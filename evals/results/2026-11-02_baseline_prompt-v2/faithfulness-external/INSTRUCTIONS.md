# Faithfulness check

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

{"errors": [{"quote": "<exact words from the summary>", "kind": "wrong|unsupported|misleading",
  "severity": "major|minor", "reason": "<one sentence, citing the transcript with a timestamp>"}]}

An empty list means the summary is faithful.
