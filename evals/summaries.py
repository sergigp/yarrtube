#!/usr/bin/env python3
"""Summary eval: generate summaries for the golden set, grade them, report.

    python3 evals/summaries.py --model qwen3:4b --model gemma3:4b
    python3 evals/summaries.py --run <run-id>          # re-grade / re-report

Steps, all cached in evals/results/<run-id>/ so an interrupted run resumes:
  1. generate: one summary per case and model (map-reduce for long videos,
     split at chapter boundaries when the video has chapters)
  2. checks:   cheap code checks on every summary (language, length, leftovers)
  3. judge:    an LLM marks each fact stated / contradicted / missing
  4. report:   per-model table + every failed critical fact

Needs Ollama (OLLAMA_URL, default http://localhost:11434) and each case's
local subtitles.srt (gitignored).
"""
import argparse, datetime, glob, hashlib, json, os, re, sys, time, unicodedata, urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))
OLLAMA = os.environ.get("OLLAMA_URL", "http://localhost:11434")
PROMPT_VERSION = "v2"
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


# --- generation (prompt v2) --------------------------------------------------

RULES = (
    "- Write in {lang}, the language of the video.\n"
    "- Plain prose, no lists or headings. Start directly with the subject, "
    "e.g. \"Three online games where…\" or \"La edad de oro del anarquismo…\".\n"
    "- Say what the video is about, its main points and how it ends: its conclusion, "
    "verdict or result (who won, what was decided).\n"
    "- If the video has several parts (several games, places, stories), say so and cover each.\n"
    "- Keep the video's own view apart from views it presents or criticizes.\n"
    "- Use only what the transcript says. Give numbers exactly as stated.\n"
    "- The transcript is auto-generated and may misspell names; use the title to correct them.\n"
    "- Ignore sponsors, ads and requests to like or subscribe."
)


def chapter_lines(chapters, start=None, end=None):
    sel = [c for c in chapters if (start is None or c["end_seconds"] > start) and (end is None or c["start_seconds"] < end)]
    return "\n".join(f"{clock(c['start_seconds'])} {c['title']}" for c in sel)


def summarize(model, case, lines):
    lang = LANG.get(case["lang"], case["lang"])
    chapters = case.get("chapters", [])
    head = f"Title: {case['title']}\nDuration: {case['duration_s'] // 60} minutes\n"
    ch_note = ("Chapters (set by the uploader; use them to see the video's structure, "
               "but summarize what is said, not the chapter titles):\n")
    pieces = chunks(lines, chapters, case["lang"])
    calls = []
    if len(pieces) == 1:
        system = "You summarize YouTube videos from their transcript.\nRules:\n" + RULES.format(lang=lang)
        user = head + (f"\n{ch_note}{chapter_lines(chapters)}\n" if chapters else "")
        user += f"\nTranscript:\n" + "\n".join(t for _, t in lines)
        user += f"\n\nNow write the summary in {lang}, 3 to 5 sentences."
        text, st = chat(model, system, user)
        return text, [st]
    parts = []
    map_system = ("You summarize one part of a long YouTube video transcript.\n"
                  f"Write in {lang}, 4 to 6 sentences of plain prose. Keep this part's key facts, names, "
                  "numbers and conclusions exactly as stated. Use only what this part says. "
                  "Ignore sponsors and requests to subscribe.")
    for n, p in enumerate(pieces, 1):
        span = f"{clock(p[0][0])}–{clock(p[-1][0])}"
        chs = chapter_lines(chapters, p[0][0], p[-1][0]) if chapters else ""
        user = (head + f"Part {n} of {len(pieces)} ({span})\n"
                + (f"\nChapters in this part:\n{chs}\n" if chs else "")
                + "\nTranscript part:\n" + "\n".join(t for _, t in p)
                + f"\n\nNow summarize this part in {lang}, 4 to 6 sentences.")
        text, st = chat(model, map_system, user)
        parts.append(f"Part {n} ({span}):\n{text}")
        calls.append(st)
    reduce_system = ("You write the final summary of a YouTube video from summaries of its consecutive parts.\n"
                     "Rules:\n" + RULES.format(lang=lang).replace("transcript", "part summaries"))
    user = (head + (f"\n{ch_note}{chapter_lines(chapters)}\n" if chapters else "")
            + "\nPart summaries, in order:\n\n" + "\n\n".join(parts)
            + f"\n\nNow write the final summary of the whole video in {lang}, 3 to 5 sentences.")
    text, st = chat(model, reduce_system, user)
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


# --- run ---------------------------------------------------------------------

def load_cases(only=None):
    cases = []
    for d in sorted(glob.glob(os.path.join(ROOT, "cases", "*"))):
        slug = os.path.basename(d)
        if only and slug not in only:
            continue
        case = json.load(open(os.path.join(d, "case.json")))
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
    ap.add_argument("--judge", default="local",
                    help=f"'local' ({JUDGE_MODEL} via Ollama) or a name for an external judge, e.g. "
                         "'claude-sonnet': writes blind inputs, ingests answers on the next run")
    a = ap.parse_args()
    judge_name = JUDGE_MODEL if a.judge == "local" else a.judge

    run_id = a.run or datetime.datetime.now().strftime("%Y-%m-%d_%H%M") + f"_prompt-{PROMPT_VERSION}"
    out = os.path.join(ROOT, "results", run_id)
    os.makedirs(out, exist_ok=True)
    meta_path = os.path.join(out, "run.json")
    meta = json.load(open(meta_path)) if os.path.exists(meta_path) else {
        "models": a.model, "prompt_version": PROMPT_VERSION, "num_ctx": NUM_CTX}
    meta.pop("judge", None)  # judges are per report now
    meta["models"] = list(dict.fromkeys(meta["models"] + a.model))
    if not meta["models"]:
        sys.exit("give at least one --model")
    json.dump(meta, open(meta_path, "w"), indent=2)
    cases = load_cases(a.case)
    slug_judge = judge_name.replace(":", "_")
    s_path = os.path.join(out, "summaries.jsonl")
    j_path = os.path.join(out, f"judgments-{slug_judge}.jsonl")

    done = {(r["case"], r["model"]) for r in jsonl(s_path)}
    for model in meta["models"]:
        for slug, case, lines in cases:
            if (slug, model) in done:
                continue
            text, calls = summarize(model, case, lines)
            append(s_path, {"case": slug, "model": model, "summary": text, "calls": calls,
                            "seconds": round(sum(c["s"] for c in calls), 1), "checks": checks(text, case)})
            print(f"generated {model:18} {slug:22} {sum(c['s'] for c in calls):6.0f}s  {len(calls)} call(s)", flush=True)

    judged = {(r["case"], r["model"]) for r in jsonl(j_path)}
    facts = {slug: case["summary"]["facts"] for slug, case, _ in cases}
    if a.judge != "local":
        if not external_judge(out, jsonl(s_path), facts, judged, j_path):
            return
    for r in jsonl(s_path) if a.judge == "local" else []:
        if (r["case"], r["model"]) in judged or r["case"] not in facts:
            continue
        verdicts, st = judge(r["summary"], facts[r["case"]])
        append(j_path, {"case": r["case"], "model": r["model"], "verdicts": verdicts, "judge_seconds": st["s"]})
        print(f"judged    {r['model']:18} {r['case']:22} " +
              " ".join(v["verdict"][0] for v in verdicts.values()), flush=True)

    report = render(run_id, meta, judge_name, cases, jsonl(s_path), jsonl(j_path))
    open(os.path.join(out, f"report-{slug_judge}.md"), "w").write(report)
    print("\n" + report + f"\nResults in {out}")


def render(run_id, meta, judge_name, cases, summaries, judgments):
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
