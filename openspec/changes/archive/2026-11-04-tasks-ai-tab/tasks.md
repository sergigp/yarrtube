# Tasks

## 1. Walking skeleton

- [x] 1.1 Add every type and signature from design.md: the `ai` members of `TaskFamily`, `TaskTab` and `TaskKind`, `taskTabs`, the `aiEnabled` parameters, `aiEnabled`, `useAiEnabled`, the AI label and icons (`Sparkles`) in `TasksView`, wired to `useAiEnabled`. Bodies keep today's behaviour: `aiEnabled` returns `false` and `summarize_video` stays in `other`. Done when `npm run check` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `puts video summaries in the ai family while AI is on`.
- [x] 2.2 `puts video summaries in the other family while AI is off`.
- [x] 2.3 `lists the ai tab between syncs and other only while AI is on`.
- [x] 2.4 `counts every task once across tabs while AI is on`.
- [x] 2.5 `gives video summaries the ai kind`.
- [x] 2.6 `lists video summaries under AI while AI is on`: drives `aiEnabled` reading the preferences.
- [x] 2.7 `shows an AI tab with no tasks while AI is on`: count 0.
- [x] 2.8 `shows no AI tab and lists summaries under Other while AI is off`.
- [x] 2.9 `shows no AI tab when preferences fail to load`.

## 3. Infrastructure adapters (TDD)

No adapter changes.

## 4. Verification

- [x] 4.1 `scripts/run-tests.sh --only web --only rust` passes.
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, turn AI on through the HTTP API, insert a pending `summarize_video` task with `sqlite3`. With the Playwright MCP browser check the AI tab shows between Syncs and Other and lists "Summarizing <title> in <source>"; turn AI off in AI settings, go back and check there's no AI tab and Other lists it. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
