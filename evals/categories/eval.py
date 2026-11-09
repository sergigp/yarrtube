#!/usr/bin/env python3
"""Categorization eval: ask a model about every video + category, score against labels.

    python3 -I evals/categories/eval.py --check
    python3 -I evals/categories/eval.py --summarize
    python3 -I evals/categories/eval.py --model qwen3:4b --limit 10     # time a few calls
    python3 -I evals/categories/eval.py --model qwen3:4b --think
    python3 -I evals/categories/eval.py --model qwen3:4b --think --batch  # one call per video, all categories
    python3 -I evals/categories/eval.py --run <run-id>                  # resume / re-score

Inputs: videos.json, labels.md (filled by hand), category-prompts.md and
prompts/categorization/<version>.md. Verdicts are cached in
results/<run-id>/verdicts.jsonl, so an interrupted run resumes. Needs Ollama
(OLLAMA_URL, default http://localhost:11434). Docs: evals/categories/README.md.
"""
import argparse, datetime, glob, hashlib, importlib.util, json, os, re, shutil, socket, subprocess, sys, tempfile, time, urllib.error, urllib.request

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(ROOT))
VIDEOS = os.path.join(ROOT, "videos.json")
LABELS = os.path.join(ROOT, "labels.md")
NONE = "None"  # the tick box for a video in no category
CATEGORY_PROMPTS = os.path.join(ROOT, "category-prompts.md")
SUBTITLES = os.path.join(ROOT, "subtitles")
PROMPT_VERSION = "v2"  # prompts/categorization/<version>.md
SUMMARY_MODEL = "qwen3:4b"  # the summaries eval's model in use: matches the summaries from prod
NUM_CTX = 4096
TEMPERATURE = 0.0
MAX_OUTPUT = 3072  # stops runaway thinking loops
BATCH_PROMPT_VERSION = "v2-batch"  # --batch's system prompt: v2 asks about one category
BATCH_NUM_CTX = 16384  # every category prompt plus thinking about all of them
BATCH_MAX_OUTPUT = 12288
BATCH_USER = """Title: {title}
Channel: {channel}
Duration: {minutes} minutes
Chapters:
{chapter_list}
Summary: {summary}

Categories:

{category_list}

Answer in JSON with one entry per category, keyed by its name: "reason", one sentence on what the video \
covers and how it fits that category, then "belongs", true or false."""
MAX_WRONG_PER_CATEGORY = 1  # the pass bar, see decisions.md


def load_summaries_eval():
    """The summaries eval, for its summarizer and helpers; -I keeps it off sys.path."""
    spec = importlib.util.spec_from_file_location("summaries_eval", os.path.join(ROOT, "..", "summaries", "eval.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


summaries_eval = load_summaries_eval()
OLLAMA, fill, jsonl, append = (summaries_eval.OLLAMA, summaries_eval.fill, summaries_eval.jsonl, summaries_eval.append)


def sha(text):
    return hashlib.sha1(text.encode()).hexdigest()[:10]


# --- inputs ------------------------------------------------------------------

def load_videos():
    return json.load(open(VIDEOS, encoding="utf-8"))


def save_videos(videos):
    tmp = VIDEOS + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(videos, f, ensure_ascii=False, indent=2)
    os.replace(tmp, VIDEOS)


def load_category_prompts(path):
    """category-prompts.md: the user message and schema, then one '## name' per category."""
    text = open(path, encoding="utf-8").read()
    parts = re.split(r"^## (.+)$", text, flags=re.M)
    sections = {name.strip(): re.findall(r"```\w*\n(.*?)\n```", body, re.S) for name, body in zip(parts[1::2], parts[2::2])}
    user, schema = sections.pop("User message draft")
    return {"user": user, "schema": json.loads(schema), "categories": {name: blocks[0] for name, blocks in sections.items()}}


def load_labels(categories, videos):
    """id -> set of categories, or None while the video isn't labeled yet."""
    if not os.path.exists(LABELS):
        sys.exit(f"missing {LABELS}")
    text = open(LABELS, encoding="utf-8").read()
    labels = {}
    for section in re.split(r"^## ", text, flags=re.M)[1:]:
        video = re.search(r"youtu\.be/([\w-]+)", section)
        boxes = re.search(r"^\[[ xX]\] .*$", section, re.M)
        if not video or not boxes:
            sys.exit(f"labels.md: no link or tick boxes under: {section.splitlines()[0]}")
        ticks = {name.strip(): mark != " " for mark, name in re.findall(r"\[([ xX])\] ([^\[\]]+?)(?= · |$)", boxes.group(0))}
        if set(ticks) != set(categories) | {NONE}:
            sys.exit(f"labels.md, {video.group(1)}: boxes don't match the categories")
        chosen = {name for name, ticked in ticks.items() if ticked and name != NONE}
        if ticks[NONE] and chosen:
            sys.exit(f"labels.md, {video.group(1)}: None ticked along with {sorted(chosen)}")
        labels[video.group(1)] = chosen if chosen or ticks[NONE] else None
    ids = {v["id"] for v in videos}
    if ids != set(labels):
        sys.exit(f"labels.md and videos.json differ: {sorted(ids ^ set(labels))}")
    return labels


def check():
    videos = load_videos()
    prompts = load_category_prompts(CATEGORY_PROMPTS)
    labels = load_labels(prompts["categories"], videos)
    pending = [v for v in videos if not v.get("summary")]
    blank = [v for v in videos if labels[v["id"]] is None]
    print(f"{len(videos)} videos, {len(prompts['categories'])} categories: {', '.join(prompts['categories'])}")
    print(f"{len(pending)} videos without a summary (run --summarize)")
    print(f"{len(blank)} videos not labeled yet")
    for v in blank:
        print(f"  {v['id']}  {v['title']}")
    for category in prompts["categories"]:
        print(f"{sum(category in (l or ()) for l in labels.values()):3} labeled {category}")


# --- missing summaries -------------------------------------------------------

def fetch_subtitles(video):
    """YouTube's captions for the video, kept locally (gitignored, see evals/.gitignore)."""
    path = os.path.join(SUBTITLES, f"{video['id']}.srt")
    if os.path.exists(path):
        return path
    os.makedirs(SUBTITLES, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(["yt-dlp", "--write-subs", "--write-auto-subs", "--sub-langs", f"{video['language']}.*",
                        "--convert-subs", "srt", "--skip-download", "-o", os.path.join(tmp, "subtitles"),
                        f"https://www.youtube.com/watch?v={video['id']}"], check=True)
        found = sorted(glob.glob(os.path.join(tmp, "*.srt")), key=len)
        if not found:
            sys.exit(f"{video['id']}: yt-dlp wrote no {video['language']} subtitles")
        shutil.move(found[0], path)
    return path


def summarize_missing(model):
    """Writes the summaries prod doesn't have into videos.json, with the prompt in use."""
    videos = load_videos()
    prompt = summaries_eval.load_prompt(summaries_eval.PROMPT_VERSION)
    for video in [v for v in videos if not v.get("summary")]:
        case = {"title": video["title"], "lang": video["language"], "duration_s": video["duration_seconds"],
                "chapters": video["chapters"]}
        lines = summaries_eval.read_srt(fetch_subtitles(video))
        video["summary"], calls = summaries_eval.summarize(model, case, lines, prompt)
        video["summary_by"] = f"{model}, summaries prompt {summaries_eval.PROMPT_VERSION}"
        save_videos(videos)
        print(f"summarized {video['id']} {sum(c['s'] for c in calls):6.0f}s  {video['title'][:60]}", flush=True)


# --- asking the model --------------------------------------------------------

def user_message(template, video, name, category_prompt):
    chapters = "\n".join(f"{summaries_eval.clock(c['start_seconds'])} {c['title']}" for c in video["chapters"])
    return fill(template, title=video["title"], channel=video["channel"], minutes=video["duration_seconds"] // 60,
                chapter_list=chapters or "(none)", summary=video["summary"], name=name, category_prompt=category_prompt)


def batch_user_message(video, categories):
    """--batch: the video once, then every category asked about, by name."""
    chapters = "\n".join(f"{summaries_eval.clock(c['start_seconds'])} {c['title']}" for c in video["chapters"])
    listed = "\n\n".join(f"### {name}\n{prompt}" for name, prompt in categories.items())
    return fill(BATCH_USER, title=video["title"], channel=video["channel"], minutes=video["duration_seconds"] // 60,
                chapter_list=chapters or "(none)", summary=video["summary"], category_list=listed)


def batch_schema(names):
    """--batch: one {reason, belongs} object per category, all required."""
    one = {"type": "object", "properties": {"reason": {"type": "string"}, "belongs": {"type": "boolean"}},
           "required": ["reason", "belongs"]}
    return {"type": "object", "properties": {name: one for name in names}, "required": list(names)}


def verdict_of(answer):
    if not isinstance(answer, dict) or not isinstance(answer.get("belongs"), bool):
        return None
    return {"belongs": answer["belongs"], "reason": str(answer.get("reason", ""))}


def json_object(text):
    try:
        return json.loads(text[text.index("{"):text.rindex("}") + 1])
    except ValueError:
        return None


def parse_verdict(text):
    """The model's verdict, or None when the answer isn't the agreed JSON."""
    return verdict_of(json_object(text))


def parse_batch(text, names):
    """--batch: category -> verdict, or None when the answer isn't usable for it."""
    answer = json_object(text)
    answer = answer if isinstance(answer, dict) else {}
    return {name: verdict_of(answer.get(name)) for name in names}


def chat(meta, schema, system, user):
    """One call: (stats, answer text) or (stats with `error`, None). A server
    that can't be reached stops the run, so resuming retries those calls."""
    body = {"model": meta["model"], "stream": False, "truncate": False, "shift": False, "think": meta["think"],
            "format": schema, "options": {"num_ctx": meta["num_ctx"], "temperature": meta["temperature"],
                                          "num_predict": meta["max_output"]},
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    request = urllib.request.Request(f"{OLLAMA}/api/chat", json.dumps(body).encode(), {"Content-Type": "application/json"})
    started = time.time()
    try:
        reply = json.load(urllib.request.urlopen(request, timeout=1800))
    except urllib.error.HTTPError as e:
        if e.code != 400:
            raise
        return {"error": e.read().decode(errors="replace")[:300], "s": round(time.time() - started, 1)}, None
    except (TimeoutError, socket.timeout):
        return {"error": "timed out", "s": round(time.time() - started, 1)}, None
    except urllib.error.URLError as e:
        sys.exit(f"can't reach Ollama at {OLLAMA}: {e.reason}. Nothing lost: resume with --run.")
    row = {"s": round(time.time() - started, 1), "in": reply.get("prompt_eval_count"), "out": reply.get("eval_count")}
    text = reply["message"]["content"].split("</think>")[-1].strip()  # qwen3 thinks even when told not to
    if reply.get("done_reason") == "length":
        return {**row, "error": f"hit the {meta['max_output']}-token output limit"}, None
    return row, text


def ask(meta, schema, system, user):
    """One call about one category; an answer the app couldn't use is a row with `error`."""
    row, text = chat(meta, schema, system, user)
    if text is None:
        return row
    verdict = parse_verdict(text)
    return {**row, **verdict} if verdict else {**row, "error": f"unusable answer: {text[:300]}"}


def ask_batch(meta, system, video, categories):
    """--batch: one call about every category in `categories`; one row per
    category, each carrying the whole call's stats."""
    row, text = chat(meta, batch_schema(list(categories)), system, batch_user_message(video, categories))
    if text is None:
        return {name: row for name in categories}
    verdicts = parse_batch(text, categories)
    return {name: {**row, **verdict} if verdict else {**row, "error": f"unusable answer: {text[:300]}"}
            for name, verdict in verdicts.items()}


def reusable_rows(run_id, meta):
    """The answers of an earlier run with the same settings, keyed for this one."""
    source = os.path.join(ROOT, "results", run_id)
    if not os.path.exists(os.path.join(source, "run.json")):
        sys.exit(f"--reuse: no run {run_id}")
    settings = json.load(open(os.path.join(source, "run.json")))
    if any(settings.get(k) != meta.get(k) for k in ("model", "think", "temperature", "num_ctx", "max_output",
                                                     "system_sha", "batch")):
        sys.exit(f"--reuse: {run_id} has other settings")
    return {(r["video"], r["category"], r["prompt_sha"]): r for r in jsonl(os.path.join(source, "verdicts.jsonl"))
            if "error" not in r}


def run_calls(out, meta, prompts, videos, limit, reuse):
    system = open(system_path(meta["prompt_version"]), encoding="utf-8").read().strip()
    path = os.path.join(out, "verdicts.jsonl")
    done = {(r["video"], r["category"]) for r in jsonl(path)}
    reused = reusable_rows(reuse, meta) if reuse else {}
    calls = 0
    for video in videos:
        if not video.get("summary"):
            continue
        if meta.get("batch"):
            pending = {name: p for name, p in prompts["categories"].items() if (video["id"], name) not in done}
            if not pending:
                continue
            if limit is not None and calls >= limit:
                return
            rows = ask_batch(meta, system, video, pending)
            call = f"{video['id']}@{time.time():.3f}"
            for name, row in rows.items():
                append(path, {"video": video["id"], "category": name, "prompt_sha": sha(pending[name]), "call": call,
                              **row})
            calls += 1
            answers = " ".join(f"{name}={'ERR' if 'error' in r else 'yes' if r['belongs'] else 'no'}"
                               for name, r in rows.items())
            row = next(iter(rows.values()))
            print(f"{row['s']:6.1f}s {row.get('out') or 0:6} tok  {video['title'][:50]}\n         {answers}", flush=True)
            continue
        for name, category_prompt in prompts["categories"].items():
            if (video["id"], name) in done:
                continue
            key = (video["id"], name, sha(category_prompt))
            if key in reused:
                append(path, reused[key])
                continue
            if limit is not None and calls >= limit:
                return
            row = ask(meta, prompts["schema"], system, user_message(prompts["user"], video, name, category_prompt))
            append(path, {"video": video["id"], "category": name, "prompt_sha": key[2], **row})
            calls += 1
            answer = "ERROR" if "error" in row else "yes" if row["belongs"] else "no"
            print(f"{row['s']:6.1f}s {answer:5} {name:26} {video['title'][:50]}", flush=True)


def system_path(version):
    return os.path.join(REPO, "prompts", "categorization", f"{version}.md")


# --- report ------------------------------------------------------------------

def percent(part, whole):
    return f"{part}/{whole}" if whole else "-"


def render(run_id, meta, prompts, videos, labels, rows):
    by_id = {v["id"]: v for v in videos}
    categories = list(prompts["categories"])
    labeled = {i for i, l in labels.items() if l is not None}
    settings = (f"model `{meta['model']}`, thinking {'on' if meta['think'] else 'off'}, prompt {meta['prompt_version']}, "
                f"temperature {meta['temperature']}, num_ctx {meta['num_ctx']}, max output {meta['max_output']}, "
                + ("one call per video (--batch)" if meta.get("batch") else "one call per video + category"))
    out = [f"# Categorization eval: {run_id}", "", settings, ""]
    total = len(videos) * len(categories)
    out += [f"{len(rows)} of {total} calls done."]
    skipped = [v for v in videos if not v.get("summary")]
    if skipped:
        out += [f"{len(skipped)} videos have no summary and weren't asked about: run `--summarize`."]
    if not labeled:
        out += ["", "**labels.md is empty: nothing is scored yet.**"]
    elif len(labeled) < len(videos):
        out += [f"Scored on the {len(labeled)} labeled videos of {len(videos)}."]
    out += ["", "| category | labeled | found | missed | wrong | precision | recall | errors | pass |",
            "|---|---|---|---|---|---|---|---|---|"]
    wrong, missed, errors = [], [], []
    for category in categories:
        found = miss = bad = 0
        answered = [r for r in rows if r["category"] == category]
        failed = [r for r in answered if "error" in r]
        for r in answered:
            if "error" in r:
                errors.append(r)
                continue
            if r["video"] not in labeled:
                continue
            expected = category in labels[r["video"]]
            found += expected and r["belongs"]
            if r["belongs"] and not expected:
                bad += 1
                wrong.append(r)
            if expected and not r["belongs"]:
                miss += 1
                missed.append(r)
        positives = sum(category in (labels[r["video"]] or ()) for r in answered)
        passed = "yes" if bad <= MAX_WRONG_PER_CATEGORY and not failed else "no"
        out.append(f"| {category} | {positives} | {found} | {miss} | {bad} | {percent(found, found + bad)} | "
                   f"{percent(found, found + miss)} | {len(failed)} | {passed} |")

    def line(r):
        video = by_id[r["video"]]
        return f"- **{r['category']}** · {video['title']} ({video['channel']}, {video['selection']}): {r.get('reason') or r.get('error')}"

    if labeled:
        out += ["", "## Wrong: said yes, labeled no", ""] + [line(r) for r in wrong]
        out += ["", "## Missed: said no, labeled yes", ""] + [line(r) for r in missed]
    if errors:
        out += ["", "## Errors: no usable answer", ""] + [line(r) for r in errors]
    out += speed(meta, rows, len(categories))
    return "\n".join(out) + "\n"


def speed(meta, rows, categories):
    """Time and output tokens per video, comparable between --batch and not.
    Output tokens don't depend on the hardware: they are what to compare
    across machines (time = tokens / the machine's tokens per second)."""
    calls = {}  # (video, call) -> stats; a --batch call's rows share its stats
    for r in rows:
        key = (r["video"], r.get("call") or r["category"])
        calls[key] = r
    per_video = {}
    for (video, _), r in calls.items():
        seconds, tokens = per_video.get(video, (0.0, 0))
        per_video[video] = (seconds + r["s"], tokens + (r.get("out") or 0))
    complete = [v for v in per_video if sum(r["video"] == v for r in rows) == categories]
    if not complete:
        return []
    seconds = sum(per_video[v][0] for v in complete) / len(complete)
    tokens = sum(per_video[v][1] for v in complete) / len(complete)
    out_tokens = sum(per_video[v][1] for v in complete)
    out_seconds = sum(per_video[v][0] for v in complete)
    rate = out_tokens / out_seconds if out_seconds else 0
    return ["", "## Speed", "",
            f"On the {len(complete)} videos asked about every category: {seconds:.1f}s and {tokens:.0f} output tokens "
            f"per video on average ({rate:.1f} tokens/s here, input included), {len(calls)} calls in all.",
            f"At 11 tokens/s (the MacBook Air with thinking) that is about {tokens / 11 / 60:.1f} min per video."]


# --- run ---------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="validate the inputs, show labeling progress and exit")
    ap.add_argument("--summarize", action="store_true", help="generate the missing summaries and exit (needs yt-dlp)")
    ap.add_argument("--model", help="model to evaluate, for a new run")
    ap.add_argument("--think", action="store_true", help="request thinking, for a new run")
    ap.add_argument("--batch", action="store_true", help="one call per video about every category, for a new run")
    ap.add_argument("--prompt", help=f"prompts/categorization/<version>.md, for a new run "
                                     f"(default {PROMPT_VERSION}, {BATCH_PROMPT_VERSION} with --batch)")
    ap.add_argument("--num-ctx", type=int, help=f"context length, for a new run (default {NUM_CTX}, "
                                                f"{BATCH_NUM_CTX} with --batch)")
    ap.add_argument("--max-output", type=int, help=f"output token cap, for a new run (default {MAX_OUTPUT}, "
                                                   f"{BATCH_MAX_OUTPUT} with --batch)")
    ap.add_argument("--temperature", type=float, default=TEMPERATURE, help="for a new run")
    ap.add_argument("--run", help="existing run id to resume / re-score")
    ap.add_argument("--reuse", help="run id whose answers are copied when settings and category prompt match")
    ap.add_argument("--limit", type=int, help="stop after this many model calls")
    a = ap.parse_args()
    if a.check:
        return check()
    if a.summarize:
        return summarize_missing(a.model or SUMMARY_MODEL)
    if not a.run and not a.model:
        sys.exit("give --model for a new run, or --run to resume one")

    a.prompt = a.prompt or (BATCH_PROMPT_VERSION if a.batch else PROMPT_VERSION)
    a.num_ctx = a.num_ctx or (BATCH_NUM_CTX if a.batch else NUM_CTX)
    a.max_output = a.max_output or (BATCH_MAX_OUTPUT if a.batch else MAX_OUTPUT)
    if a.max_output > a.num_ctx:
        sys.exit("--max-output can't be over --num-ctx")
    think = "think-on" if a.think else "think-off"
    run_id = a.run or (datetime.datetime.now().strftime("%Y-%m-%d_%H%M") +
                       f"_prompt-{a.prompt}_{a.model.replace(':', '-')}_{think}" + ("_batch" if a.batch else ""))
    out = os.path.join(ROOT, "results", run_id)
    meta_path = os.path.join(out, "run.json")
    snapshot = os.path.join(out, "category-prompts.md")
    if not os.path.exists(meta_path):  # a run keeps the category prompts it started with
        if a.run:
            sys.exit(f"no run {a.run}")
        if not os.path.exists(system_path(a.prompt)):
            sys.exit(f"no prompt {system_path(a.prompt)}")
        os.makedirs(out)
        shutil.copy(CATEGORY_PROMPTS, snapshot)
        system = open(system_path(a.prompt), encoding="utf-8").read().strip()
        json.dump({"model": a.model, "think": a.think, "prompt_version": a.prompt, "system_sha": sha(system),
                   "num_ctx": a.num_ctx, "temperature": a.temperature, "max_output": a.max_output, "batch": a.batch},
                  open(meta_path, "w"), indent=2)
    meta = json.load(open(meta_path))
    prompts = load_category_prompts(snapshot)
    videos = load_videos()
    labels = load_labels(prompts["categories"], videos)

    run_calls(out, meta, prompts, videos, a.limit, a.reuse)

    report = render(run_id, meta, prompts, videos, labels, jsonl(os.path.join(out, "verdicts.jsonl")))
    open(os.path.join(out, "report.md"), "w", encoding="utf-8").write(report)
    print("\n" + report + f"\nResults in {out}")


if __name__ == "__main__":
    main()
