# Patch debt: lines of ours inside upstream files

Only allowed to go down. Each line names the reason and whether it was offered upstream.

| File | Lines | What | Why | Offered upstream |
|---|---|---|---|---|
| `apps/photocraft-web/Cargo.toml` | 3 | `page-control` feature and optional `serde_json` | Turns on the page control bridge | Not yet (generic hook, task UV-PHOTO-photocraft-upstreamoffer) |
| `apps/photocraft-web/src/main.rs` | 3 | declares `mod page_control` behind the feature | Same | Not yet |
| `apps/photocraft-web/src/web.rs` | 4 | `app = page_control::attach(app, ctx)` behind the feature | Gives the bridge the app's control channel | Not yet |
| `crates/ui-egui/src/state.rs` | 5 | `Panels.community_links` switch, default on | An embedding app credits PhotoCraft in its own About and hides the Discord and ArtCraft links | Not yet (same pattern as upstream's `menu_bar` and `rail` switches) |
| `crates/ui-egui/src/canvas.rs` | 2 (+4 re-indented) | start-screen links behind the switch | Same | Not yet |
| `crates/ui-egui/src/dialogs.rs` | 2 (+5 re-indented) | About links behind the switch | Same | Not yet |
| `crates/ui-egui/src/workspace_ui.rs` | 1 | workspaces do not save the switch | Same, as for `menu_bar` and `rail` | Not yet |

Total: 20 added lines in 7 files (9 more re-indented). New files of ours (no debt): `apps/photocraft-web/src/page_control.rs`,
`c0/photocraft-host/`.
