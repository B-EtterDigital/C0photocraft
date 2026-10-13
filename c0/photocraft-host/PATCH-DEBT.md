# Patch debt: lines of ours inside upstream files

Only allowed to go down. Each line names the reason and whether it was offered upstream.

| File | Lines | What | Why | Offered upstream |
|---|---|---|---|---|
| `apps/photocraft-web/Cargo.toml` | 3 | `page-control` feature and optional `serde_json` | Turns on the page control bridge | Not yet (generic hook, task UV-PHOTO-photocraft-upstreamoffer) |
| `apps/photocraft-web/src/main.rs` | 3 | declares `mod page_control` behind the feature, and for the crate's own tests (the bridge's rules for which requests ask for a frame are plain Rust and tested natively) | Same | Not yet |
| `apps/photocraft-web/src/web.rs` | 7 (1 replaced) | 4: `app = page_control::attach(app, ctx)` behind the feature. 3 (and 1 replaced): the web runner gets a name (`let runner = eframe::WebRunner::new();`, then `let result = runner` where the constructor stood) and is handed to the bridge behind the feature (`page_control::runner(&runner)`) | Gives the bridge the app's control channel, and the runner it needs to answer a request in a pass of its own instead of on the display's next frame (about one a second in a window that is not presented) | Not yet |
| `crates/ui-egui/src/state.rs` | 13 | 5: `Panels.community_links` switch, default on. 8: `Panels.toolbar_floating` and `Panels.toolbar_float` (where the palette sits, whether it is pinned), both `#[serde(default)]`, with their defaults | An embedding app credits PhotoCraft in its own About and hides the Discord and ArtCraft links. The toolbar's third mode: a palette over the canvas | Not yet (same pattern as upstream's `menu_bar`, `rail` and `toolbar_double` switches) |
| `crates/ui-egui/src/panels.rs` | 4 (1 blank more, 1 replaced) | 1: `pub mod toolbar_float;` declares our `panels/toolbar_float.rs`. 1 (replacing 1): the toolbar's left panel is built into `let panel = …` instead of shown at once. 1: `toolbar_float::host(app, ui.ctx(), (w, bx, m), panel).show(` draws the unchanged tool closure in that panel or in the palette. 1: `toolbar_float::mode_menu` on the header chevron. 1: `toolbar_float::settle` after the tools | The floating toolbar reuses upstream's tool buttons, flyouts and footer as they are: the closure that draws them is not touched or re-indented, only what it is drawn in | Not yet |
| `crates/ui-egui/src/retouch_ui.rs` | 0 (2 replaced) | `set_source` keeps the ⌥-clicked point as it is instead of rounding it to a whole pixel (`[x, y]` for `[x.round(), y.round()]`, twice) | **Upstream bug fix candidate.** The source point was rounded when set and the clone offset rounded again when the stroke began, so the copied patch could sit up to a pixel off the source point (measured 0.8 px; with the pointer at the centre of a pixel for both clicks and the source right of or below the dab, every pixel came from one pixel further right and down). One rounding is left, half a pixel at most. Clone Stamp and Healing Brush. Tests: `crates/ui-egui/tests/c0_clone_offset.rs` (3 of its 4 tests fail on upstream's lines) | Not yet (a plain bug fix, worth offering) |
| `crates/ui-egui/src/canvas.rs` | 2 (+4 re-indented) | start-screen links behind the switch | Same | Not yet |
| `crates/ui-egui/src/dialogs.rs` | 2 (+5 re-indented) | About links behind the switch | Same | Not yet |
| `crates/ui-egui/src/workspace_ui.rs` | 1 | workspaces do not save the switch | Same, as for `menu_bar` and `rail` | Not yet |
| `crates/ui-egui/src/theme.rs` | 5 (2 removed) | 1: `pub mod host;` declares our `theme/host.rs`. 1 (replacing 1): `apply` takes its tokens through `host::resolve`. 3 (replacing 1): the gate `#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]` moves from the whole `live` module onto its `SystemTime` import, `LiveTokens` and `impl LiveTokens` | The host sets PhotoCraft's colours and radii at run time (`ui.set {themeTokens}`), in the browser release build. `apply` is the one place every preset change, new document and first frame goes through, so the palette stays in force there and egui's widget visuals follow it. `live::apply_overrides` and its `color` helper are upstream's own token list and parser; only the file polling stays debug-only | Not yet (a generic run-time token override) |
| `crates/ui-egui/src/control.rs` | 1 | one arm at the top of `dispatch`: a method starting with `c0.` goes to `c0_generative::handle` | The host's capture, selection mask, place, open-from-bytes and cancel-dialog requests (`ui-egui/src/c0_generative.rs`, ours) need the app, which only `control::handle` has; one arm keeps every rule in our file and lets a native test run them | Not yet |
| `crates/ui-egui/src/lib.rs` | 1 | `pub mod c0_generative;` | Declares our file | Not yet |

Total: 42 added lines in 12 files (1 more blank, 9 more re-indented, 4 replaced in place, 2 removed).
Was 40 added lines in 10 files before the `c0.*` requests (2026-10-13, +1 in `control.rs`, +1 in `lib.rs`, both new to the list).
Was 25 added lines in 8 files before the toolbar and clone changes (2026-10-13): +3 in `web.rs`, +8 in `state.rs`, +4 in `panels.rs`
(new to the list), and `retouch_ui.rs` (new to the list) with two lines replaced and none added.

New files of ours (no debt): `apps/photocraft-web/src/page_control.rs`,
`apps/photocraft-web/src/page_control/bridge.rs`, `crates/ui-egui/src/c0_generative.rs`, `crates/ui-egui/src/theme/host.rs`,
`crates/ui-egui/src/panels/toolbar_float.rs`, `crates/ui-egui/tests/c0_*.rs`, `c0/photocraft-host/`.

## Couplings without an edited line (re-check on every pull)

| Ours | Leans on | Check |
|---|---|---|
| `theme/host.rs` `put` | the canvas dot texture's cache key in `canvas.rs` `dots` (`("canvas-dots", "<ThemeKind>")`); it is dropped so a new `canvas_dot` colour is drawn | Not covered by a test. If upstream changes the key, the dots keep the colour they were first drawn in |
| `theme/host.rs` `put` | the id `"photocraft-theme"` under which `theme::apply` publishes the tokens | `host_palette_*` tests in `theme/host.rs` |
| `theme/host.rs` `sort` | `theme::live::apply_overrides` (the list of token names and the value shapes) | `host_palette_applies_to_tokens_and_widget_visuals_and_reports_each_key` |
| `panels/toolbar_float.rs` `content_height` | the spacing inside `panels::toolbar` (row height, group dividers, footer) and its private `visible_sections` and `chips_height` | Not covered by a test of its own. If upstream changes the spacing, the palette is a little too tall or its tool column scrolls; `c0_toolbar_floating.rs` still checks every tool button is drawn and clickable |
| `panels/toolbar_float.rs` `Float::show` | `PhotocraftApp.last_canvas_rect` (the canvas area of the last frame, as upstream's own floating panels use it) | `floating_toolbar_gives_the_canvas_the_freed_width` |
| `page_control/bridge.rs` `Pump` | eframe's web runner: `WebRunner::add_event_listener` lending the app runner to a listener, and `AppRunner::logic()` running one unpainted pass (eframe 0.36.2) | Compiled for `wasm32` by the web build; not run by any test. `page_control::tests::the_web_shell_hands_its_runner_to_the_bridge` checks `web.rs` still gives the bridge the runner |
| `page_control.rs` `reads_only` | which control requests change nothing on screen | `c0_control_passes.rs` `a_read_is_answered_by_one_pass_that_asks_for_no_frame` runs each listed request |
| `c0_generative.rs` `capture` | `photocraft_compose::render_reduced_rect` (the renderer behind `doc.render` and thumbnails) and `Rect`'s `u32` sizes | `capture_*` and `max_side_downsizes_*` in `c0_generative_bridge.rs` |
| `c0_generative.rs` `place`, `open_*` | `Session::edit` (one history step), `Layer::new`, `LayerMask`, `PhotocraftApp::open_bytes` (synchronous when `background_jobs` is off, as in the web build) and the `file_dialog` field | `place_*`, `clip_to_selection_*`, `open_file_*`, `cancel_file_dialog_*` in `c0_generative_bridge.rs` |
| `c0_generative.rs` `image_surface` | `photocraft_algo::resample::resize_surface_in_canvas` repeating edge pixels outwards (without it a 1-pixel image fades at the border) | `place_scales_the_image_to_cover_the_rect` |
| the host app (C0X) | panel switches, theme tokens, `ui.inspect` fields, command ids, menu entries | `crates/ui-egui/tests/c0_host_contract.rs` |
