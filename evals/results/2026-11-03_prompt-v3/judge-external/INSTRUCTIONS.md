# Judge task

You check a video summary against reference facts about the video.
For each fact, give one verdict:
- "stated": the summary explicitly says the fact's main claim. Paraphrases count, and so does
  a different language (the summary may be in Spanish, the facts in English). When a fact
  bundles several details, its main claim being said is enough. If you have to infer or
  read between the lines, it is "missing", not "stated".
- "contradicted": the summary says something incompatible with the fact.
- "missing": the summary does not address the fact.
Judge only the summary against the facts. Do not use your own knowledge of the topic.
Give a one-sentence reason for each verdict, quoting the summary when you can.

Read your input file (named in your task). It has a summary and the facts. Judge only from
that file: do not open any other file in this directory or the repository.

Write your answer to `out/<key>.json`, where <key> is the input file's name without
extension, as JSON exactly in this shape and nothing else:

{"verdicts": [{"id": "<fact id>", "verdict": "stated|contradicted|missing", "reason": "<one sentence>"}]}

One verdict per fact id, every fact id present.
