Web-only change: the TDD "full suite" for each cycle is `npm run test` in `web/` (not `cargo test`). Copy and visuals follow the canvas https://claude.ai/artifact/HKASVSHWj1auGYgsBCmcuQ and the specs in this change.

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md with trivial bodies, and wire them in:
  - `lib/settingsStatus.ts`, `lib/aiChanges.ts`, `lib/playlistPreview.ts`, `syncStatusLine` in `lib/plexStatus.ts`, `couldntSave` in `lib/errorMessage.ts`;
  - the components `SettingsCard`, `PageHeader`, `Segmented`, `SaveBar`, `CopyButton` and `PromptDisclosure`;
  - replace `SettingsSection` with `SettingsCard`;
  - render the new components in `SettingsView`, `PreferencesView`, `AiSettingsView` and `PlexSettingsView` without changing behaviour yet.

  Done when `npm run check` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `it shows Off, On · Ollama and On · Anthropic for AI`: AI nav status
- [x] 2.2 `it shows Not set up, Not checked yet, Connected and Can't reach Plex`: Plex nav status
- [x] 2.3 `it shows each section's status under its name`: nav renders statuses, hidden on mobile
- [ ] 2.4 `it shows the renamed switches with their descriptions`: General cards, inline help, no tooltips
- [ ] 2.5 `it links categories to where they are also used`: "Also used by" links
- [ ] 2.6 `it disables More about while categories are off`
- [ ] 2.7 `it shows Couldn't save when a toggle fails`
- [ ] 2.8 `it names changed settings once each, in page order`
- [ ] 2.9 `it counts unsaved changes in singular and plural`
- [ ] 2.10 `it shows the AI features switch beside the title`
- [ ] 2.11 `it describes each provider option`: option cards
- [ ] 2.12 `it shows no save bar until a field changes`
- [ ] 2.13 `it lists the unsaved changes in the save bar`
- [ ] 2.14 `it saves only the changed fields and shows Saved by the title`
- [ ] 2.15 `it discards changes without a request`
- [ ] 2.16 `it shows a rejected save's error in the save bar`
- [ ] 2.17 `it shows Anthropic effort as a segmented choice outside Advanced`
- [ ] 2.18 `it marks the prompt default or edited`: Prompt disclosure for both cards
- [ ] 2.19 `it names the missing Anthropic key and the restart`
- [ ] 2.20 `it puts last and next run on one line`
- [ ] 2.21 `it says when a failed sync retries`
- [ ] 2.22 `it orders rows and drops the watched one when removing`
- [ ] 2.23 `it shows the server badge, version, libraries and last checked`
- [ ] 2.24 `it tells what to check when Plex is unreachable`
- [ ] 2.25 `it shows Retry now on a failed sync`
- [ ] 2.26 `it saves a playlist choice and updates the preview`
- [ ] 2.27 `it turns categories on from the Plex page`
- [ ] 2.28 `it shows the setup card when Plex isn't configured`
- [ ] 2.29 `it copies a variable name`

## 3. Infrastructure adapters (TDD)

None: no adapter changes.

## 4. Verification

- [ ] 4.1 Update `e2e-tests/tests/preferences.spec.js`, `aiSettings.spec.js` and `plexSettings.spec.js` to the new names. Renames: "Enable categories" → "Categories", "Show …" → the new switch names, "Enable AI" → "AI features", "Save" → "Save changes", "Watch state" → "Watched status", "Labels & playlists" → "Categories to Plex".
- [ ] 4.2 `scripts/run-tests.sh --lint --web --rust`
- [ ] 4.3 `scripts/run-tests.sh --e2e`, because e2e specs changed.
- [ ] 4.4 Manual (agent):
  - Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  - With the Playwright MCP browser, open `/settings/general`, `/settings/ai` and `/settings/plex`, and check:
    - the nav statuses;
    - "Also used by" navigates;
    - the AI save bar appears on edit, Discard clears it, and Save changes sends only the changed fields (network log);
    - Plex: the setup card with Copy (unless Plex env is set), otherwise the badge, sync rows, Turn on categories and the preview;
    - at 390px width there is no horizontal scroll.
  - Screenshot each page in light and dark.
  - Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): visual judgement against the canvas in light and dark themes; the Plex sections against a real Plex server (Connected / Can't reach Plex, Retry now).
