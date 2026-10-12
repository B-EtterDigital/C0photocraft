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
| `crates/ui-egui/src/theme.rs` | 5 (2 removed) | 1: `pub mod host;` declares our `theme/host.rs`. 1 (replacing 1): `apply` takes its tokens through `host::resolve`. 3 (replacing 1): the gate `#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]` moves from the whole `live` module onto its `SystemTime` import, `LiveTokens` and `impl LiveTokens` | The host sets PhotoCraft's colours and radii at run time (`ui.set {themeTokens}`), in the browser release build. `apply` is the one place every preset change, new document and first frame goes through, so the palette stays in force there and egui's widget visuals follow it. `live::apply_overrides` and its `color` helper are upstream's own token list and parser; only the file polling stays debug-only | Not yet (a generic run-time token override) |

Total: 25 added lines in 8 files (9 more re-indented, 2 removed). New files of ours (no debt):
`apps/photocraft-web/src/page_control.rs`, `crates/ui-egui/src/theme/host.rs`, `c0/photocraft-host/`.

## Couplings without an edited line (re-check on every pull)

| Ours | Leans on | Check |
|---|---|---|
| `theme/host.rs` `put` | the canvas dot texture's cache key in `canvas.rs` `dots` (`("canvas-dots", "<ThemeKind>")`); it is dropped so a new `canvas_dot` colour is drawn | Not covered by a test. If upstream changes the key, the dots keep the colour they were first drawn in |
| `theme/host.rs` `put` | the id `"photocraft-theme"` under which `theme::apply` publishes the tokens | `host_palette_*` tests in `theme/host.rs` |
| `theme/host.rs` `sort` | `theme::live::apply_overrides` (the list of token names and the value shapes) | `host_palette_applies_to_tokens_and_widget_visuals_and_reports_each_key` |
