# Patch debt: lines of ours inside upstream files

Only allowed to go down. Each line names the reason and whether it was offered upstream.

| File | Lines | What | Why | Offered upstream |
|---|---|---|---|---|
| `apps/photocraft-web/Cargo.toml` | 3 | `page-control` feature and optional `serde_json` | Turns on the page control bridge | Not yet (generic hook, task UV-PHOTO-photocraft-upstreamoffer) |
| `apps/photocraft-web/src/main.rs` | 3 | declares `mod page_control` behind the feature | Same | Not yet |
| `apps/photocraft-web/src/web.rs` | 4 | `app = page_control::attach(app, ctx)` behind the feature | Gives the bridge the app's control channel | Not yet |

Total: 10 lines in 3 files. New files of ours (no debt): `apps/photocraft-web/src/page_control.rs`,
`c0/photocraft-host/`.
