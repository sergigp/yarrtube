# yarrtube web UI

A small React + Vite single-page app that browses playlists, a playlist's
videos, and pending/in-progress tasks. Built with `npm run build` and
embedded into the `yarrtube` binary at compile time — see `DEVELOPMENT.md`
at the repo root.

```bash
npm ci
npm run dev    # local dev server, proxies /api to localhost:8080
npm run build  # produces dist/, embedded by `cargo build`
```
