#!/usr/bin/env python3
"""Summary eval: generate summaries for the golden set, grade them, report.

    python3 -I evals/summaries/eval.py --model qwen3:4b --model gemma3:4b
    python3 -I evals/summaries/eval.py --run <run-id>          # re-grade / re-report

Steps, all cached in evals/summaries/results/<run-id>/ so an interrupted run resumes:
  1. generate: one summary per case and model (map-reduce for long videos,
     split at chapter boundaries when the video has chapters)
  2. checks:   cheap code checks on every summary (language, length, leftovers)
  3. judge:    an LLM marks each fact stated / contradicted / missing
  4. report:   per-model table + every failed critical fact

Prompts come from prompts/summaries/<version>.md. Needs Ollama (OLLAMA_URL,
default http://localhost:11434) and each case's local subtitles.srt
(gitignored). Docs: evals/summaries/README.md.
"""
import argparse, datetime, glob, hashlib, json, os, re, sys, time, unicodedata, urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))
OLLAMA = os.environ.get("OLLAMA_URL", "http://localhost:11434")
PROMPT_VERSION = "v3"  # prompts/summaries/<version>.md; the one in use
NUM_CTX = 12288
INPUT_BUDGET_TOKENS = 9000  # leaves room in NUM_CTX for thinking + answer
CHARS_PER_TOKEN = {"en": 4.3}  # measured on the golden set; other languages ~3.4
LANG = {"es": "Spanish", "en": "English", "ca": "Catalan", "fr": "French", "de": "German"}
JUDGE_MODEL = os.environ.get("JUDGE_MODEL", "qwen3:8b")
JUDGE_CTX = 4096  # the judge reads one summary + facts: short, so a bigger model fits

# --- transcript --------------------------------------------------------------

TS = re.compile(r"(\d+):(\d+):(\d+)[,.](\d+)\s*-->\s*(\d+):(\d+):(\d+)[,.](\d+)")
TAG = re.compile(r"<[^>]+>")


def read_srt(path):
    """Caption lines with their start time; mirrors TranscriptText::from_srt
    (tags stripped, consecutive repeated lines from rolling captions collapsed)."""
    lines, start = [], 0.0
    for raw in open(path, encoding="utf-8", errors="replace").read().replace("\r", "").split("\n"):
        line = raw.strip()
        m = TS.search(line)
        if m:
            h, mi, s, ms = m.groups()[:4]
            start = int(h) * 3600 + int(mi) * 60 + int(s) + int(ms) / 1000
            continue
        if not line or line.isdigit():
            continue
        line = TAG.sub("", line).strip()
        if line and (not lines or lines[-1][1] != line):
            lines.append((start, line))
    return lines


def tokens(text, lang):
    return len(text) / CHARS_PER_TOKEN.get(lang, 3.4)


def clock(seconds):
    s = int(seconds)
    return f"{s // 3600}:{s % 3600 // 60:02d}:{s % 60:02d}" if s >= 3600 else f"{s // 60}:{s % 60:02d}"


def chunks(lines, chapters, lang):
    """Consecutive pieces of the transcript, each under the input budget.
    Cuts only at chapter starts when the video has chapters, packing whole
    chapters together; a chapter too long on its own is cut by size."""
    budget = INPUT_BUDGET_TOKENS
    if tokens("\n".join(t for _, t in lines), lang) <= budget:
        return [lines]
    cuts = [c["start_seconds"] for c in chapters[1:]]
    sections, cur = [], []
    for ln in lines:
        if cuts and ln[0] >= cuts[0] and cur:
            sections.append(cur)
            cur = []
        while cuts and ln[0] >= cuts[0]:
            cuts.pop(0)
        cur.append(ln)
    sections.append(cur)
    pieces = []
    for sec in sections:  # cut oversized sections by size
        part = []
        for ln in sec:
            if part and tokens("\n".join(t for _, t in part + [ln]), lang) > budget:
                pieces.append(part)
                part = []
            part.append(ln)
        pieces.append(part)
    packed = []
    for p in pieces:  # pack small neighbours together
        if packed and tokens("\n".join(t for _, t in packed[-1] + p), lang) <= budget:
            packed[-1] = packed[-1] + p
        else:
            packed.append(p)
    return packed


# --- ollama ------------------------------------------------------------------

def chat(model, system, user, fmt=None, think=False, ctx=NUM_CTX, max_out=None, timeout=3600):
    options = {"num_ctx": ctx, "temperature": 0.3}
    if max_out:
        options["num_predict"] = max_out  # stops runaway thinking loops
    body = {"model": model, "stream": False, "think": think, "options": options,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    if fmt:
        body["format"] = fmt
    req = urllib.request.Request(f"{OLLAMA}/api/chat", json.dumps(body).encode(),
                                 {"Content-Type": "application/json"})
    t0 = time.time()
    r = json.load(urllib.request.urlopen(req, timeout=timeout))
    text = r["message"]["content"].split("</think>")[-1].strip()  # qwen3 thinks even when told not to
    truncated = r.get("prompt_eval_count", 0) >= ctx - 64
    return text, {"s": round(time.time() - t0, 1), "in": r.get("prompt_eval_count"),
                  "out": r.get("eval_count"), "truncated": truncated}


# --- generation ----------------------------------------------------------------

PROMPTS = os.path.join(os.path.dirname(os.path.dirname(ROOT)), "prompts", "summaries")
PLACEHOLDER = re.compile(r"\{(\w+)\}")


def load_prompt(version):
    """Sections of prompts/summaries/<version>.md: '## name' heading -> template."""
    text = open(os.path.join(PROMPTS, f"{version}.md"), encoding="utf-8").read()
    parts = re.split(r"^## (.+)$", text, flags=re.M)
    return {name.strip(): body.strip("\n") for name, body in zip(parts[1::2], parts[2::2])}


def fill(template, **values):
    """Fills {name} placeholders in one pass, so braces inside values are left alone."""
    return PLACEHOLDER.sub(lambda m: str(values[m.group(1)]), template)


def chapter_lines(chapters, start=None, end=None):
    sel = [c for c in chapters if (start is None or c["end_seconds"] > start) and (end is None or c["start_seconds"] < end)]
    return "\n".join(f"{clock(c['start_seconds'])} {c['title']}" for c in sel)


def summarize(model, case, lines, prompt):
    lang = LANG.get(case["lang"], case["lang"])
    chapters = case.get("chapters", [])
    head = {"title": case["title"], "minutes": case["duration_s"] // 60, "lang": lang}

    def chapter_block(section, start=None, end=None):
        listed = chapter_lines(chapters, start, end) if chapters else ""
        return f"\n{fill(prompt[section], chapter_list=listed)}\n" if listed else ""

    pieces = chunks(lines, chapters, case["lang"])
    if len(pieces) == 1:
        user = fill(prompt["user"], **head, chapters=chapter_block("chapters"),
                    transcript="\n".join(t for _, t in lines))
        text, st = chat(model, fill(prompt["system"], lang=lang), user)
        return text, [st]
    parts, calls = [], []
    for n, p in enumerate(pieces, 1):
        span = f"{clock(p[0][0])}–{clock(p[-1][0])}"
        user = fill(prompt["map user"], **head, part=n, parts=len(pieces), span=span,
                    chapters=chapter_block("map chapters", p[0][0], p[-1][0]),
                    transcript="\n".join(t for _, t in p))
        text, st = chat(model, fill(prompt["map system"], lang=lang), user)
        parts.append(fill(prompt["reduce part"], part=n, span=span, summary=text))
        calls.append(st)
    user = fill(prompt["reduce user"], **head, chapters=chapter_block("chapters"),
                part_summaries="\n\n".join(parts))
    text, st = chat(model, fill(prompt["reduce system"], lang=lang), user)
    return text, calls + [st]


# --- code checks -------------------------------------------------------------

STOP = {"es": set("el la los las de que y en un una es por con para se del al lo como más pero".split()),
        "en": set("the and of to is in that it for with on as this are was but be by".split())}


def checks(summary, case):
    words = re.findall(r"[a-záéíóúñü]+", summary.lower())
    scores = {l: sum(w in s for w in words) for l, s in STOP.items()}
    detected = max(scores, key=scores.get) if any(scores.values()) else None
    sentences = [s for s in re.split(r"(?<=[.!?])\s+", summary.strip()) if s]
    res = {
        "language": case["lang"] not in STOP or detected == case["lang"],
        "length_3_to_5_sentences": 3 <= len(sentences) <= 5,
        "no_leftovers": not re.search(r"<think>|</think>|https?://|subscri|suscr|here'?s a summary", summary, re.I),
        "direct_start": not re.match(r"\s*(the|this|in this) video|\s*(el|este|en este) v[ií]deo", summary, re.I),
    }
    return res


# --- judge -------------------------------------------------------------------

JUDGE_SYSTEM = """You check a video summary against reference facts about the video.
For each fact, give one verdict:
- "stated": the summary explicitly says the fact's main claim. Paraphrases count, and so does
  a different language (the summary may be in Spanish, the facts in English). When a fact
  bundles several details, its main claim being said is enough. If you have to infer or
  read between the lines, it is "missing", not "stated".
- "contradicted": the summary says something incompatible with the fact.
- "missing": the summary does not address the fact.
Judge only the summary against the facts. Do not use your own knowledge of the topic.
Give a one-sentence reason for each verdict, quoting the summary when you can."""

JUDGE_FORMAT = {"type": "object", "required": ["verdicts"], "properties": {"verdicts": {
    "type": "array", "items": {"type": "object", "required": ["id", "verdict", "reason"], "properties": {
        "id": {"type": "string"}, "reason": {"type": "string"},
        "verdict": {"type": "string", "enum": ["stated", "contradicted", "missing"]}}}}}}


def judge_user_prompt(summary, facts):
    listed = "\n".join(f"- id: {f['id']}\n  fact: {f['fact']}"
                       + (f"\n  common mistake: {f['trap']}" if f.get("trap") else "")
                       + (f"\n  note: {f['note']}" if f.get("note") else "") for f in facts)
    return f"Summary:\n{summary}\n\nFacts:\n{listed}\n\nGive one verdict per fact id."


def judge(summary, facts):
    user = judge_user_prompt(summary, facts)
    for attempt in range(2):
        try:
            text, st = chat(JUDGE_MODEL, JUDGE_SYSTEM, user, fmt=JUDGE_FORMAT, think=True,
                            ctx=JUDGE_CTX, max_out=2500, timeout=1200)
        except TimeoutError:
            text = "timed out"
            continue
        try:
            got = {v["id"]: v for v in json.loads(text[text.index("{"):text.rindex("}") + 1])["verdicts"]}
            if all(f["id"] in got for f in facts):
                return {f["id"]: got[f["id"]] for f in facts}, st
        except (ValueError, KeyError):
            pass
    raise RuntimeError(f"judge gave no usable verdicts: {text[:300]}")


# --- external judge (e.g. Claude subagents) ----------------------------------
# Blind hand-off: one input file per summary, named by a hash so the judge
# can't tell which model wrote it. Whoever judges writes out/<key>.json as
# {"verdicts": [{"id", "verdict", "reason"}, ...]}; the next run ingests it.

EXTERNAL_INSTRUCTIONS = """# Judge task

{system}

Read your input file (named in your task). It has a summary and the facts. Judge only from
that file: do not open any other file in this directory or the repository.

Write your answer to `out/<key>.json`, where <key> is the input file's name without
extension, as JSON exactly in this shape and nothing else:

{{"verdicts": [{{"id": "<fact id>", "verdict": "stated|contradicted|missing", "reason": "<one sentence>"}}]}}

One verdict per fact id, every fact id present.
"""


def external_key(case, model):
    return hashlib.sha1(f"{case}|{model}".encode()).hexdigest()[:10]


def external_judge(out, summaries, facts, judged, j_path):
    d = os.path.join(out, "judge-external")
    os.makedirs(os.path.join(d, "in"), exist_ok=True)
    os.makedirs(os.path.join(d, "out"), exist_ok=True)
    open(os.path.join(d, "INSTRUCTIONS.md"), "w").write(EXTERNAL_INSTRUCTIONS.format(system=JUDGE_SYSTEM))
    pending = []
    for r in summaries:
        if (r["case"], r["model"]) in judged or r["case"] not in facts:
            continue
        key = external_key(r["case"], r["model"])
        answer = os.path.join(d, "out", f"{key}.json")
        if os.path.exists(answer):
            got = {v["id"]: v for v in json.load(open(answer))["verdicts"]}
            missing = [f["id"] for f in facts[r["case"]] if f["id"] not in got]
            if missing:
                sys.exit(f"{answer}: no verdict for {missing}")
            verdicts = {f["id"]: got[f["id"]] for f in facts[r["case"]]}
            append(j_path, {"case": r["case"], "model": r["model"], "verdicts": verdicts})
            print(f"ingested  {r['model']:18} {r['case']:22} " +
                  " ".join(v["verdict"][0] for v in verdicts.values()), flush=True)
        else:
            open(os.path.join(d, "in", f"{key}.md"), "w").write(judge_user_prompt(r["summary"], facts[r["case"]]))
            pending.append(key)
    if pending:
        print(f"\n{len(pending)} summaries waiting for the external judge in {d}:")
        print("\n".join(f"  in/{k}.md" for k in pending))
    return not pending


# --- faithfulness (external judge with the transcript) ------------------------
# Facts only measure coverage: a summary can state every fact and still add a
# wrong claim ("Claude Dots"). This check gives the judge the full transcript
# and asks for every claim in the summary that the video doesn't support.

FAITH_INSTRUCTIONS = """# Faithfulness check

You check whether a short video summary is faithful to the video. Your input file has
the video title, the summary, and the path of the transcript (YouTube auto-captions:
names may be misspelled, lines may repeat). Read the whole transcript, in parts if long.

List every claim in the summary that is not faithful to the video:
- "wrong": the video says something different (wrong name, number, result, attribution,
  or the video's view confused with a view it criticizes).
- "unsupported": the video doesn't say it (invented detail, overgeneralization).
- "misleading": each part is true, but put together they give a wrong picture of the
  video (e.g. a side mention presented as the main topic).
Severity: "major" if it changes what a viewer would take away from the video; "minor" for
a small detail. Do not flag omissions, style, translation, or captions' spelling mistakes
the summary corrects. Do not use your own knowledge: only the transcript counts.

Do not open any file other than your input file and the transcript it names.
Write your answer to `out/<key>.json` (<key> = input file name without extension), JSON only:

{{"errors": [{{"quote": "<exact words from the summary>", "kind": "wrong|unsupported|misleading",
  "severity": "major|minor", "reason": "<one sentence, citing the transcript with a timestamp>"}}]}}

An empty list means the summary is faithful.
"""


def external_faithfulness(out, summaries, cases, f_path):
    d = os.path.join(out, "faithfulness-external")
    for sub in ("in", "out", "transcripts"):
        os.makedirs(os.path.join(d, sub), exist_ok=True)
    open(os.path.join(d, "INSTRUCTIONS.md"), "w").write(FAITH_INSTRUCTIONS.format())
    by_slug = {slug: (case, lines) for slug, case, lines in cases}
    done = {(r["case"], r["model"]) for r in jsonl(f_path)}
    pending = []
    for r in summaries:
        if (r["case"], r["model"]) in done or r["case"] not in by_slug:
            continue
        case, lines = by_slug[r["case"]]
        key = external_key(r["case"], r["model"])
        answer = os.path.join(d, "out", f"{key}.json")
        if os.path.exists(answer):
            errors = json.load(open(answer))["errors"]
            append(f_path, {"case": r["case"], "model": r["model"], "errors": errors})
            print(f"ingested  {r['model']:18} {r['case']:22} {len(errors)} error(s)", flush=True)
            continue
        transcript = os.path.join(d, "transcripts", f"{r['case']}.txt")  # gitignored
        if not os.path.exists(transcript):
            open(transcript, "w").write("\n".join(f"[{clock(t)}] {x}" for t, x in lines))
        open(os.path.join(d, "in", f"{key}.md"), "w").write(
            f"Title: {case['title']}\nTranscript: {os.path.abspath(transcript)}\n\nSummary:\n{r['summary']}\n")
        pending.append(key)
    if pending:
        print(f"\n{len(pending)} summaries waiting for the faithfulness check in {d}:")
        print("\n".join(f"  in/{k}.md" for k in pending))
    return not pending


# --- run ---------------------------------------------------------------------

CASE_FIELDS = {"youtube_id": str, "url": str, "title": str, "channel": str, "published": str,
               "lang": str, "duration_s": int, "chapters": list, "description": str, "summary": dict}
FACT_FIELDS = {"id": str, "fact": str, "critical": bool}
REVIEW_FIELDS = {"reviewed_on": str, "by": str, "note": str}


def validate_case(slug, case):
    """Problems with a case.json against the format in evals/README.md."""
    errs = []

    def typed(obj, fields, where):
        for k, t in fields.items():
            if not isinstance(obj.get(k), t):
                errs.append(f"{where}{k}: missing or not {t.__name__}")

    typed(case, CASE_FIELDS, "")
    if errs:
        return errs
    if case["url"] != f"https://www.youtube.com/watch?v={case['youtube_id']}":
        errs.append("url doesn't match youtube_id")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", case["published"]):
        errs.append("published: not YYYY-MM-DD")
    for c in case["chapters"]:
        typed(c, {"title": str, "start_seconds": (int, float), "end_seconds": (int, float)}, "chapters[].")
    facts = case["summary"].get("facts", [])
    if not facts:
        errs.append("summary.facts: empty")
    ids = [f.get("id") for f in facts]
    if len(ids) != len(set(ids)):
        errs.append("summary.facts: duplicate ids")
    for f in facts:
        typed(f, FACT_FIELDS, f"facts[{f.get('id')}].")
    if not any(f.get("critical") for f in facts):
        errs.append("summary.facts: no critical fact")
    review = case["summary"].get("review")
    if review is not None:
        typed(review, REVIEW_FIELDS, "review.")
        hv = review.get("human_verdicts")
        if hv is not None:
            if not isinstance(hv.get("run"), str):
                errs.append("review.human_verdicts.run: missing")
            for model, verdicts in hv.items():
                if model == "run":
                    continue
                for fid, v in verdicts.items():
                    if fid not in ids or v not in ("stated", "contradicted", "missing"):
                        errs.append(f"review.human_verdicts.{model}.{fid}: unknown fact or verdict")
    return errs


def load_cases(only=None):
    cases = []
    for d in sorted(glob.glob(os.path.join(ROOT, "cases", "*"))):
        slug = os.path.basename(d)
        if only and slug not in only:
            continue
        case = json.load(open(os.path.join(d, "case.json")))
        errs = validate_case(slug, case)
        if errs:
            sys.exit(f"{slug}/case.json: " + "; ".join(errs))
        case["title"] = unicodedata.normalize("NFC", case["title"])
        srt = os.path.join(d, "subtitles.srt")
        if not os.path.exists(srt):
            sys.exit(f"missing {srt} (subtitles are local only, see evals/.gitignore)")
        cases.append((slug, case, read_srt(srt)))
    return cases


def jsonl(path):
    return [json.loads(l) for l in open(path)] if os.path.exists(path) else []


def append(path, row):
    with open(path, "a") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", action="append", default=[], help="model to evaluate (repeatable)")
    ap.add_argument("--run", help="existing run id to resume / re-report")
    ap.add_argument("--case", action="append", help="limit to these case slugs")
    ap.add_argument("--prompt", default=PROMPT_VERSION, help="prompts/summaries/<version>.md for a new run")
    ap.add_argument("--check", action="store_true", help="only validate the cases and exit")
    ap.add_argument("--judge", default="local",
                    help=f"'local' ({JUDGE_MODEL} via Ollama) or a name for an external judge, e.g. "
                         "'claude-sonnet': writes blind inputs, ingests answers on the next run")
    a = ap.parse_args()
    judge_name = JUDGE_MODEL if a.judge == "local" else a.judge
    if a.check:
        for slug, case, _ in load_cases(a.case):
            print(f"ok {slug}")
        return

    run_id = a.run or datetime.datetime.now().strftime("%Y-%m-%d_%H%M") + f"_prompt-{a.prompt}"
    out = os.path.join(ROOT, "results", run_id)
    os.makedirs(out, exist_ok=True)
    meta_path = os.path.join(out, "run.json")
    meta = json.load(open(meta_path)) if os.path.exists(meta_path) else {
        "models": a.model, "prompt_version": a.prompt, "num_ctx": NUM_CTX}
    meta.pop("judge", None)  # judges are per report now
    meta["models"] = list(dict.fromkeys(meta["models"] + a.model))
    if not meta["models"]:
        sys.exit("give at least one --model")
    json.dump(meta, open(meta_path, "w"), indent=2)
    cases = load_cases(a.case)
    prompt = load_prompt(meta["prompt_version"])  # a resumed run keeps its own prompt
    slug_judge = judge_name.replace(":", "_")
    s_path = os.path.join(out, "summaries.jsonl")
    j_path = os.path.join(out, f"judgments-{slug_judge}.jsonl")

    done = {(r["case"], r["model"]) for r in jsonl(s_path)}
    for model in meta["models"]:
        for slug, case, lines in cases:
            if (slug, model) in done:
                continue
            text, calls = summarize(model, case, lines, prompt)
            append(s_path, {"case": slug, "model": model, "summary": text, "calls": calls,
                            "seconds": round(sum(c["s"] for c in calls), 1), "checks": checks(text, case)})
            print(f"generated {model:18} {slug:22} {sum(c['s'] for c in calls):6.0f}s  {len(calls)} call(s)", flush=True)

    judged = {(r["case"], r["model"]) for r in jsonl(j_path)}
    facts = {slug: case["summary"]["facts"] for slug, case, _ in cases}
    f_path = os.path.join(out, f"faithfulness-{slug_judge}.jsonl")
    if a.judge != "local":
        facts_done = external_judge(out, jsonl(s_path), facts, judged, j_path)
        faith_done = external_faithfulness(out, jsonl(s_path), cases, f_path)
        if not (facts_done and faith_done):
            return
    for r in jsonl(s_path) if a.judge == "local" else []:
        if (r["case"], r["model"]) in judged or r["case"] not in facts:
            continue
        verdicts, st = judge(r["summary"], facts[r["case"]])
        append(j_path, {"case": r["case"], "model": r["model"], "verdicts": verdicts, "judge_seconds": st["s"]})
        print(f"judged    {r['model']:18} {r['case']:22} " +
              " ".join(v["verdict"][0] for v in verdicts.values()), flush=True)

    report = render(run_id, meta, judge_name, cases, jsonl(s_path), jsonl(j_path), jsonl(f_path))
    open(os.path.join(out, f"report-{slug_judge}.md"), "w").write(report)
    print("\n" + report + f"\nResults in {out}")


def render(run_id, meta, judge_name, cases, summaries, judgments, faith=()):
    facts = {slug: {f["id"]: f for f in case["summary"]["facts"]} for slug, case, _ in cases}
    J = {(j["case"], j["model"]): j["verdicts"] for j in judgments}
    S = {(s["case"], s["model"]): s for s in summaries}
    out = [f"# Summary eval: prompt {meta['prompt_version']}, judge {judge_name}", "",
           "| model | contradicted | critical stated | facts stated | code checks | avg time |",
           "|---|---|---|---|---|---|"]
    fails = []
    for model in meta["models"]:
        con = crit = crit_ok = tot = ok = chk = chk_ok = 0
        secs = []
        for slug in facts:
            if (slug, model) not in J:
                continue
            s = S[(slug, model)]
            secs.append(s["seconds"])
            chk += len(s["checks"])
            chk_ok += sum(s["checks"].values())
            for fid, v in J[(slug, model)].items():
                f = facts[slug][fid]
                tot += 1
                ok += v["verdict"] == "stated"
                con += v["verdict"] == "contradicted"
                if f["critical"]:
                    crit += 1
                    crit_ok += v["verdict"] == "stated"
                if f["critical"] and v["verdict"] != "stated" or v["verdict"] == "contradicted":
                    fails.append((model, slug, fid, v))
        if tot:
            out.append(f"| {model} | {con}/{tot} | {crit_ok}/{crit} | {ok}/{tot} | {chk_ok}/{chk} | "
                       f"{sum(secs) / len(secs):.0f}s |")
    out += ["", "## Failed critical facts and contradictions", ""]
    for model, slug, fid, v in fails:
        out.append(f"- **{model}** · {slug} · `{fid}` → {v['verdict']}: {v['reason']}")
    if faith:
        F = {(f["case"], f["model"]): f["errors"] for f in faith}
        out += ["", "## Faithfulness: claims the video doesn't support", "",
                "| model | major | minor | faithful summaries |", "|---|---|---|---|"]
        for model in meta["models"]:
            errs = [e for (slug, m), es in F.items() if m == model for e in es]
            n = sum(1 for (slug, m) in F if m == model)
            clean = sum(1 for (slug, m), es in F.items() if m == model and not es)
            out.append(f"| {model} | {sum(e['severity'] == 'major' for e in errs)} | "
                       f"{sum(e['severity'] == 'minor' for e in errs)} | {clean}/{n} |")
        out.append("")
        for (slug, m), es in sorted(F.items()):
            for e in es:
                out.append(f"- **{m}** · {slug} · {e['severity']} {e['kind']}: \"{e['quote']}\" → {e['reason']}")
    agree, disagree = human_agreement(run_id, cases, judgments)
    if agree + len(disagree):
        out += ["", f"## Judge vs human verdicts: {agree}/{agree + len(disagree)} agree", ""]
        out += [f"- {m} · {slug} · `{fid}`: human {h}, judge {j}" for m, slug, fid, h, j in disagree]
    out += ["", "## Code checks failed", ""]
    for (slug, model), s in sorted(S.items()):
        bad = [k for k, ok in s["checks"].items() if not ok]
        if bad:
            out.append(f"- {model} · {slug}: {', '.join(bad)}")
    return "\n".join(out) + "\n"


def human_agreement(run_id, cases, judgments):
    """Compare the judge with the verdicts a viewer gave in case.json review.human_verdicts."""
    J = {(j["case"], j["model"]): j["verdicts"] for j in judgments}
    agree, disagree = 0, []
    for slug, case, _ in cases:
        human = case["summary"].get("review", {}).get("human_verdicts", {})
        if human.get("run") != run_id:  # verdicts are about that run's summaries
            continue
        for model, verdicts in human.items():
            if model == "run" or (slug, model) not in J:
                continue
            for fid, h in verdicts.items():
                j = J[(slug, model)][fid]["verdict"]
                if h == j:
                    agree += 1
                else:
                    disagree.append((model, slug, fid, h, j))
    return agree, disagree


if __name__ == "__main__":
    main()
