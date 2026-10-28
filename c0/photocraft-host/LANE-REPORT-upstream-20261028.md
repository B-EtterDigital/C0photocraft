# PhotoCraft upstream pull — 2026-10-28

Branch: `fix`. Fork base: `dfeb6db06a43ae9241c1ae90d3befc27804d5ad9` (`origin/c0/main`).
Upstream: `52f338e2df44ac73c0bf231fa8d3f8cd496641ec`. Merge base: `a0ff3a01f2723fb29e4aa5bd035987a544f32b72`.
Merge, never rebase; no push or GitHub login. Work performed only on this fleet host.

## Conflict resolutions

| Conflicted path | Resolution |
|---|---|
| `.github/workflows/ci.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/docs.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/freebsd.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/packaging-lint.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/perf-nightly.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/release.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `.github/workflows/windows-arm64.yml` | Accepted upstream in the merge; removed in the mandatory separate CI/trademark cleanup commit. Fork policy deletion, no application hook. |
| `apps/photocraft-web/src/web.rs` | Re-applied the feature-gated page bridge after upstream's new drag/place-aware app constructor; kept upstream drag listeners and services. |
| `crates/engine/src/lib.rs` | Moved DocState uid and lifecycle calls into upstream doc_state.rs and session/documents.rs; kept only the c0 module declaration and Session filter field here. |
| `crates/ui-egui/src/canvas.rs` | Moved out: tab/filter seam to canvas/tabs.rs; Home links to canvas/start_screen.rs; ants clock to canvas/ants.rs. This file equals upstream. |
| `crates/ui-egui/src/canvas/tool_feedback.rs` | Moved clock to canvas/ants.rs. This file equals upstream and delegates to the new device-pixel renderer. Its former explicit pixel snapping is supplied by that renderer, exercised by the adapted c0_ants mesh regression. |
| `crates/ui-egui/src/control.rs` | Moved the single c0 dispatch arm to control/dispatch.rs. This file equals upstream. |
| `crates/ui-egui/src/lib.rs` | Re-applied three c0 module declarations alongside upstream's reorganized modules and compatibility re-exports. |
| `crates/ui-egui/src/menus.rs` | Moved the visible-document cycle seam to menus/invoke.rs. This file equals upstream. |
| `crates/ui-egui/src/panels.rs` | Moved the palette wrapper/menu/settle hooks to panels/toolbar.rs. Kept only the toolbar_float module declaration here; the upstream RTL-aware panel constructor is preserved. |
| `crates/ui-egui/src/retouch_ui.rs` | Kept deleted. Ported both unrounded clone/heal source-coordinate replacements to tools/retouch_ui.rs. The original c0_clone_offset regression remains. |
| `crates/ui-egui/src/theme.rs` | Re-applied runtime host palette resolution and feature gates around file polling only; retained upstream token definitions, parser and egui 0.37 theme IDs. |

Every fork hunk in upstream-owned source was read in its final module against the upstream tree.
The upstream lockfile was accepted unchanged; no manual lock editing. No host feature was dropped. The host filter now activates through upstream's set_active method, so its new active-document generation and mask-colour synchronization remain correct. A regression checks that filtering out the active document advances that generation.
The old ants pixel-snap wrapper is subsumed by upstream's device-pixel renderer; the fixed-tick
clock and tick-boundary repaint scheduling remain, with the regression adapted to compare the
actual new renderer's mesh. No deleted upstream source file was resurrected.

## Patch debt

Before: ledger lists 63 added lines in 16 files, plus 1 blank, 9 re-indented, 14 replaced and 4 removed.
That ledger total was overstated: recounting c0/main source hooks against merge base with the same nonblank,
indentation-normalized comparison gives **54 added, 15 replaced, 2 removed, 2 blanks**.
After resolution, source hooks: **54 nonblank added lines in 20 files, 14 replaced, 2 removed, 2 additional blanks**;
indentation-only changes excluded. See PATCH-DEBT.md for each seam and moved-path accounting.
The 3-line delegation for filtered tiling moves its policy into c0_sessions.rs and preserves upstream's
stable tile positions. Added debt is flat under a consistent counter, replacements fall by one, and the total remains below the old listed ceiling of 63. All conflicted paths, including zero-debt
moved files and CI policy removals, are now accounted for in that ledger.
The inherited README fork/branding notice was also missing from the old ledger: zero added, six
replaced and three removed, unchanged in this pull. Including it, all retained upstream-file debt
is **54 added in 21 files, 20 replaced, 5 removed, 2 blanks**, versus a consistent baseline of
**54 added in 17 files, 21 replaced, 5 removed, 2 blanks**.

## Verification

All Cargo commands run under `smoac-slot run gate 3 3600 --` with `CARGO_BUILD_JOBS=4` and
`CARGO_TARGET_DIR=/srv/smoac/build/sow-fix-photocraft-target`. UI/engine tests use `RUST_TEST_THREADS=4`.
Disk is checked before every build; the wrapper stops above 88%. Host-managed gate caps and reservations changed during this lane; this lane never adjusted them.
Several admission waits exceeded 15 minutes; the last verification batch is still queued.

| Command / context | Result | Load before (1/5/15 min) | Load after | Disk before -> after |
|---|---|---|---|---|
| `cargo test -p photocraft-web` | **20 passed, 0 failed, 0 ignored** | 5.26 / 9.62 / 14.48 at queue entry; 10.48 / 13.59 / 14.35 last pre-admission sample | 10.01 / 13.39 / 14.18 | 81% at queue entry; 83% at admission -> 83% |
| UI first build (old importer fixtures) | compile error E0631; repaired | 8.48 / 11.20 / 12.90 | 26.96 / 18.36 / 15.34 | 83% -> 84% |
| UI first complete run (narrow Pro fixture) | 4353 passed, 29 failed, 19 ignored; Pro fixture repaired | 13.09 / 14.13 / 14.81 | 18.51 / 21.35 / 18.47 | 83% -> 84% |
| Pure upstream UI, same full command | 4278 passed, 28 failed, 19 ignored | 18.63 / 20.25 / 19.03 | 19.44 / 21.86 / 20.42 | 85% -> 86% |
| Fork UI after merge repairs, same full command | 4354 passed, 28 upstream-red, 19 ignored; zero fork-only failures | 19.44 / 21.86 / 20.42 | 16.82 / 16.92 / 18.44 | 86% -> 86% |
| Engine `cargo test -p photocraft-engine --no-fail-fast` | **2629 passed, 0 failed, 20 ignored** | 16.82 / 16.92 / 18.44 | 9.73 / 12.92 / 16.31 | 86% -> 82% |
| New minimum-coordinate regression before fix | 1 failed: confirmed signed-absolute-value overflow | 13.02 / 13.43 / 15.13 | 16.42 / 14.28 / 15.35 | 83% -> 83% |
| Final full fork UI after overflow fix | **4355 passed, 28 upstream-red, 19 ignored; zero fork-only failures** | 16.42 / 14.28 / 15.35 | 18.61 / 16.29 / 15.93 | 83% -> 83% |
| Final web tests | **20 passed, 0 failed, 0 ignored** | 18.61 / 16.29 / 15.93 | 16.90 / 16.03 / 15.85 | 83% -> 83% |
| wasm default check | **pass** | 16.90 / 16.03 / 15.85 | 16.51 / 15.97 / 15.83 | 83% -> 83% |
| wasm `--features page-control,heif` | **pass** | 16.51 / 15.97 / 15.83 | 16.31 / 15.93 / 15.82 | 83% -> 83% |
| First strict native clippy | Three C0 diagnostics; repaired redundant casts and fixed-size RGBA iteration | 16.31 / 15.93 / 15.82 | 13.39 / 15.26 / 15.60 | 83% -> 83% |

The complete UI command on both trees is `cargo test -p photocraft-ui-egui --no-fail-fast`.
The comparison keys failures by test binary and full test name; every final fork failure also
fails on the untouched upstream worktree at `52f338e2`. That worktree remains clean.

Fork fixture repairs: the three import callbacks now forward `&ImportOptions` through
`import_with_options`; upstream replaced their old scalar SVG-depth argument. The Pro tab-filter
fixture now has an 1800 px viewport instead of 1100 px, leaving room for upstream's full-title tabs.
Every assertion remains; overflow is separately tested by upstream's tab tests.

Host-specific UI proof: all **66 integration tests** pass (clone offset 4, control passes 5,
generative bridge 21, host contract 10, sessions 7, synthetic input 5, floating toolbar 14), plus
**11 unit tests** (ants clock 6, host palette 3, stable filtered tiling 1, minimum-coordinate rejection 1). The upstream ants tests
also pass, including whole device-pixel thickness, equal rectangle edges at fractional scale and
antialiased slanted lines: this proves the old point-snap wrapper is unnecessary in the new renderer.
The fixed-tick phase and repaint scheduling remain in force. No crate/manifest exists under c0/;
its Rust host tests live in photocraft-ui-egui and photocraft-web.

The page bridge now constructs ControlRequest through its constructor, initializing upstream's new
dispatch state. Rectangle validation uses unsigned_abs so i64::MIN is rejected without panic; the
new test reproduced the original overflow before the fix and passes for both axes afterward.
The later lint-only cleanup removes redundant u32 casts and uses fixed-size four-channel chunks
with the same per-pixel conversion; it changes only the C0 generative module.

Final native/wasm clippy, repeat wasm checks after lint-only cleanup, targeted generative retests,
layer check and offscreen palette captures are still queued. The edited host modules/harness pass rustfmt --check;
git diff --check is clean. No release web page was built: release wasm size unmeasured, cap
unchanged at **25165824 bytes**.

### Confirmed upstream-red UI tests

All 28 below panic in epaint's fonts.rs with `No glyph for …` on this host. The same full command,
thread limit and optional-font environment reproduce them on pure upstream; no fallback policy or
upstream font code was changed. Counts above retain Cargo's nonzero UI exit status (101).

- `src/lib.rs::i18n::live_tests::a_control_request_changes_the_drawing_language_in_the_same_frame`
- `src/lib.rs::i18n::live_tests::an_explicit_language_survives_restart_on_a_different_system_language`
- `src/lib.rs::i18n::live_tests::dynamic_brush_sections_draw_in_the_selected_language`
- `src/lib.rs::i18n::live_tests::first_launch_renders_the_system_language_without_overwriting_auto`
- `src/lib.rs::i18n::live_tests::korean_glyphs_remain_available_after_repeated_language_switches`
- `src/lib.rs::i18n::live_tests::preferences_preview_does_not_commit_until_applied_and_survives_reload`
- `src/lib.rs::input::hold_keys::tests::space_while_editing_type_is_not_the_hand_tool`
- `src/lib.rs::panels::brush_panel::tests::brush_sections_paint_simplified_chinese_labels`
- `src/lib.rs::panels::layer_row_ui::edit_tests::text_thumbnail_opens_inline_editing_and_typing_is_one_undo_step`
- `src/lib.rs::panels::layer_row_ui::tests::collapse_all_groups_from_the_panel_menu_and_it_is_saved`
- `src/lib.rs::panels::layer_row_ui::tests::long_names_never_run_under_the_indicators`
- `src/lib.rs::panels::layer_row_ui::tests::new_group_items_in_the_panel_menu_open_the_dialog`
- `src/lib.rs::panels::layer_row_ui::tests::the_fx_triangle_hides_and_shows_the_effects_rows`
- `src/lib.rs::tools::type_tool::canvas_tests::a_drag_just_outside_the_typed_text_moves_it`
- `src/lib.rs::tools::type_tool::canvas_tests::empty_ime_events_never_delete_the_selection`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_captures_press_and_rejects_collapsed_geometry`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_free_scale_center_scale_move_skew_and_rotation`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_noop_cancel_and_stale_capture`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_preserves_paragraph_resize_pivot_ime_and_shortcuts`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_scales_without_reflow_or_leaving_type`
- `src/lib.rs::tools::type_tool::canvas_tests::temporary_type_transform_visibility_and_cursors`
- `src/lib.rs::tools::type_tool::canvas_tests::the_move_reach_is_on_screen_and_leaves_other_type_alone`
- `src/lib.rs::tools::type_tool::canvas_tests::type_selection_highlight_visible_with_command_held`
- `src/lib.rs::tools::type_tool::font_picker_tests::localized_font_names_label_the_menu_but_pick_the_family`
- `src/lib.rs::tools::type_tool::ime_tests::actual_font_search_owns_stock_input_without_a_speculative_custom_request`
- `src/lib.rs::tools::type_tool::ime_tests::actual_screen_picker_modal_retires_the_type_fence_and_return_rearms_it`
- `src/lib.rs::tools::vector_ui::paint_tests::shape_paint_hex_input_keeps_partial_text_and_preserves_alpha`
- `tests/it/main.rs::camera_raw_localization::all_registered_languages_render_camera_raw_and_switch_without_resetting_view_or_filter`

## What upstream adds

Features: editable font size/leading/tracking presets; font favourites, recent/class/similar filters and
live preview; New Document artboards and custom Artboard from Layers sizes; Color Sampler points;
History snapshots and non-linear history options; action editing and ATN import/export; export presets;
more PSD smart-filter/frame-layer fidelity; XCF import; IFF/Netpbm export and improved Save As choices;
printer discovery and shared panel docking; two Soft themes and update status/check commands.

Performance: band-by-band raw development avoids a full floating-point RGB frame; faster PSD and
native preview export; bounded selection feather/morphology work; redundant smart-object resampling
removed; faster Photomerge, stack modes and Camera Raw shadows/highlights; GPU memory budgeting and
cached inline previews. These describe upstream changes; this lane did not re-benchmark them.

Reliability: deeply nested XML/SVG is rejected; malformed plug-in code sections and selection masks
return errors; automation paths reject Windows device names and non-image overwrites. Autosave
sidecars replace atomically and recovery listings avoid duplicate entries. Export As no longer freezes
on large layered PSDs. Native saves report directory-flush failures, PSB is selected by encoded size,
and clone/history/CMYK/fill/smart-filter save fidelity improves. Undo/redo fixes cover Camera Raw,
Liquify previews and focused numeric fields, Puppet Warp pins, crop boxes and history memory accounting.

## UI and host integration

- Home: Auto Show Home Screen can be disabled; an empty workspace then replaces Home. Our community-links switch still gates Home and About links, default true.
- Menus: new commands and dynamic choices remain discoverable through engine.commands / ui.menu.list. Full-title tab sizing changes overflow behavior; the host filter now feeds the shared fitter.
- Panels: shared craft_ui docking replaces native docking code. Floating toolbar retains upstream tool buttons, RTL panel direction and the new chevron in every theme. The palette height estimate now includes the all-theme chevron, Edit Toolbar, Quick Mask and Screen Mode controls. Palette sizing still depends on toolbar spacing; native viewport interaction needs a visual follow-up beyond headless tests.
- Theme: egui 0.37 uses Role and Id::unique; our palette and harness IDs were updated. The canvas-dot texture cache key still matches our invalidation key. Runtime tokens resolve through upstream's parser and apply path.
- Community links: switch remains default-on and is omitted from workspace saves, as before. **Pre-existing clash:** dock::apply deserializes missing Panels fields with their defaults, preserving only menu_bar/rail explicitly; selecting a saved workspace can therefore restore community_links to true. Confirmed in both merge base dock.rs:746–750 and upstream dock.rs:426–430. This lane preserves that behavior; the embedding host can reassert its switch after workspace changes, and a separate fix would add a preservation seam/test. The separate cleanup removes workflows and docs/brand material except its license. The compiled titlebar icon now lives under assets/app-icon, not docs/brand and can still show upstream branding if the host enables that bar; no blank replacement at docs/brand is required.
- Agent server: control dispatch moved to control/dispatch.rs; c0 methods stay ahead of upstream methods. eframe 0.37 still exposes WebRunner::add_event_listener and AppRunner::logic. New command rows include needsDocument/disabledReason/keywords. Native control authentication/path changes are upstream; the page bridge retains its own pass pump and request limits.

## New upstream commands and menu items

Type > Anti-Alias > None (`type.antiAlias.none`) is the one added static menu catalog row.
New/expanded implemented families include History snapshots/options and step commands, action editing,
ATN load/save, layer-style contours and ASL libraries, type path attachment/area conversion, color samplers,
font favourites/library filters, export presets, layer XMP, guide editing/ruler origins, printer discovery,
face detection and update checks. Some IDs existed as disabled catalog entries before acquiring implementations.
The source inventory below also includes newly explicit shell/tool aliases (not all are new visible tools).

New literal command declarations and shell aliases (including constant-valued IDs), compared with the merge-base registry. Generated families and aliases are included where declared as tuples. The engine command list remains the runtime authority.

- `actions.assignSet` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.conditional` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.dialog` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.insert` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.load` — `crates/engine/src/cmd/atn_cmds.rs`
- `actions.photoshopStep` — `crates/engine/src/cmd/atn_cmds.rs`
- `actions.save` — `crates/engine/src/cmd/atn_cmds.rs`
- `actions.setColor` — `crates/engine/src/cmd/actions_cmds.rs`
- `actions.stopMessage` — `crates/engine/src/cmd/atn_cmds.rs`
- `brush.mixer.clean` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.mixer.load` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.mixer.sample` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.presets.importImage` — `crates/engine/src/cmd/preset_import_cmds.rs`
- `channel.target.layerMask` — `crates/engine/src/cmd/channel_cmds.rs`
- `document.recent.add` — `crates/engine/src/cmd/document_preset_cmds.rs`
- `document.recent.list` — `crates/engine/src/cmd/document_preset_cmds.rs`
- `edit.fillHistory` — `crates/engine/src/cmd/fill_key_cmds.rs`
- `edit.installedProfiles` — `crates/engine/src/installed_profiles.rs`
- `file.export.presets.delete` — `crates/engine/src/cmd/export_preset_cmds.rs`
- `file.export.presets.get` — `crates/engine/src/cmd/export_preset_cmds.rs`
- `file.export.presets.list` — `crates/engine/src/cmd/export_preset_cmds.rs`
- `file.export.presets.save` — `crates/engine/src/cmd/export_preset_cmds.rs`
- `file.print.page` — `crates/engine/src/cmd/print_cmds.rs`
- `file.print.printers` — `crates/engine/src/cmd/print_cmds.rs`
- `file.printers` — `crates/engine/src/cmd/print_cmds.rs`
- `help.checkForUpdates` — `crates/engine/src/cmd/updates_cmds.rs`
- `history.clear` — `crates/engine/src/cmd/history_menu_cmds.rs`
- `history.deleteSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.newSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.options` — `crates/engine/src/cmd/history_options_cmds.rs`
- `history.redo` — `crates/engine/src/cmd/history_cmds.rs`
- `history.renameSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.selectSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.selectState` — `crates/engine/src/cmd/history_select_cmds.rs`
- `history.setSource` — `crates/engine/src/cmd/history_source_cmds.rs`
- `history.stepBackward` — `crates/engine/src/cmd/history_menu_cmds.rs`
- `history.stepForward` — `crates/engine/src/cmd/history_menu_cmds.rs`
- `history.undo` — `crates/engine/src/cmd/history_cmds.rs`
- `image.detectFaces` — `crates/engine/src/cmd/face_cmds.rs`
- `layer.artboard.addBeside` — `crates/engine/src/cmd/artboard_cmds.rs`
- `layer.fill.pattern.set` — `crates/engine/src/cmd/pattern_fill_cmds.rs`
- `layer.layerStyle.contours.delete` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.list` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.load` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.new` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.save` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.list` — `crates/engine/src/cmd/layer_list_cmds.rs`
- `layer.mergeGroup` — `crates/engine/src/cmd.rs`
- `layer.smartFilter.transfer` — `crates/engine/src/cmd/smart_cmds.rs`
- `layer.toggleLockTransparency` — `crates/engine/src/cmd/extra_cmds.rs`
- `layer.xmpMetadata` — `crates/engine/src/cmd/layer_xmp_cmds.rs`
- `layerComp.reorder` — `crates/engine/src/cmd/comps_cmds.rs`
- `paint.artHistoryBrush` — `crates/engine/src/cmd/retouch_cmds.rs`
- `select.loadLuminosity` — `crates/engine/src/cmd/channel_cmds.rs`
- `style.presets.exportAsl` — `crates/engine/src/cmd/style_asl_cmds.rs`
- `style.presets.importAsl` — `crates/engine/src/cmd/style_asl_cmds.rs`
- `tools.firstBrush` — `crates/engine/src/cmd/brush_key_cmds.rs`
- `tools.lastBrush` — `crates/engine/src/cmd/brush_key_cmds.rs`
- `tools.nextBrush` — `crates/engine/src/cmd/brush_key_cmds.rs`
- `tools.previousBrush` — `crates/engine/src/cmd/brush_key_cmds.rs`
- `tools.rotateBrush` — `crates/engine/src/cmd/brush_key_cmds.rs`
- `tools.select.addAnchorPoint` — `crates/engine/src/shell_commands.rs`
- `tools.select.artHistoryBrush` — `crates/engine/src/shell_commands.rs`
- `tools.select.artboard` — `crates/engine/src/shell_commands.rs`
- `tools.select.backgroundEraser` — `crates/engine/src/shell_commands.rs`
- `tools.select.blur` — `crates/engine/src/shell_commands.rs`
- `tools.select.brush` — `crates/engine/src/shell_commands.rs`
- `tools.select.burn` — `crates/engine/src/shell_commands.rs`
- `tools.select.cloneStamp` — `crates/engine/src/shell_commands.rs`
- `tools.select.colorReplacement` — `crates/engine/src/shell_commands.rs`
- `tools.select.colorSampler` — `crates/engine/src/shell_commands.rs`
- `tools.select.contentAwareMove` — `crates/engine/src/shell_commands.rs`
- `tools.select.convertPoint` — `crates/engine/src/shell_commands.rs`
- `tools.select.count` — `crates/engine/src/shell_commands.rs`
- `tools.select.crop` — `crates/engine/src/shell_commands.rs`
- `tools.select.curvaturePen` — `crates/engine/src/shell_commands.rs`
- `tools.select.customShape` — `crates/engine/src/shell_commands.rs`
- `tools.select.deleteAnchorPoint` — `crates/engine/src/shell_commands.rs`
- `tools.select.directSelection` — `crates/engine/src/shell_commands.rs`
- `tools.select.dodge` — `crates/engine/src/shell_commands.rs`
- `tools.select.ellipseMarquee` — `crates/engine/src/shell_commands.rs`
- `tools.select.ellipseShape` — `crates/engine/src/shell_commands.rs`
- `tools.select.eraser` — `crates/engine/src/shell_commands.rs`
- `tools.select.eyedropper` — `crates/engine/src/shell_commands.rs`
- `tools.select.frame` — `crates/engine/src/shell_commands.rs`
- `tools.select.freeformPen` — `crates/engine/src/shell_commands.rs`
- `tools.select.gradient` — `crates/engine/src/shell_commands.rs`
- `tools.select.hand` — `crates/engine/src/shell_commands.rs`
- `tools.select.healing` — `crates/engine/src/shell_commands.rs`
- `tools.select.historyBrush` — `crates/engine/src/shell_commands.rs`
- `tools.select.horizontalTypeMask` — `crates/engine/src/shell_commands.rs`
- `tools.select.lasso` — `crates/engine/src/shell_commands.rs`
- `tools.select.line` — `crates/engine/src/shell_commands.rs`
- `tools.select.magicEraser` — `crates/engine/src/shell_commands.rs`
- `tools.select.magicWand` — `crates/engine/src/shell_commands.rs`
- `tools.select.magneticLasso` — `crates/engine/src/shell_commands.rs`
- `tools.select.mixerBrush` — `crates/engine/src/shell_commands.rs`
- `tools.select.move` — `crates/engine/src/shell_commands.rs`
- `tools.select.note` — `crates/engine/src/shell_commands.rs`
- `tools.select.objectSelection` — `crates/engine/src/shell_commands.rs`
- `tools.select.paintBucket` — `crates/engine/src/shell_commands.rs`
- `tools.select.patch` — `crates/engine/src/shell_commands.rs`
- `tools.select.pathSelection` — `crates/engine/src/shell_commands.rs`
- `tools.select.patternStamp` — `crates/engine/src/shell_commands.rs`
- `tools.select.pen` — `crates/engine/src/shell_commands.rs`
- `tools.select.pencil` — `crates/engine/src/shell_commands.rs`
- `tools.select.perspectiveCrop` — `crates/engine/src/shell_commands.rs`
- `tools.select.polygon` — `crates/engine/src/shell_commands.rs`
- `tools.select.polygonLasso` — `crates/engine/src/shell_commands.rs`
- `tools.select.quickSelection` — `crates/engine/src/shell_commands.rs`
- `tools.select.rectMarquee` — `crates/engine/src/shell_commands.rs`
- `tools.select.rectangle` — `crates/engine/src/shell_commands.rs`
- `tools.select.redEye` — `crates/engine/src/shell_commands.rs`
- `tools.select.remove` — `crates/engine/src/shell_commands.rs`
- `tools.select.rotateView` — `crates/engine/src/shell_commands.rs`
- `tools.select.ruler` — `crates/engine/src/shell_commands.rs`
- `tools.select.sharpen` — `crates/engine/src/shell_commands.rs`
- `tools.select.singleColumnMarquee` — `crates/engine/src/shell_commands.rs`
- `tools.select.singleRowMarquee` — `crates/engine/src/shell_commands.rs`
- `tools.select.slice` — `crates/engine/src/shell_commands.rs`
- `tools.select.sliceSelect` — `crates/engine/src/shell_commands.rs`
- `tools.select.smudge` — `crates/engine/src/shell_commands.rs`
- `tools.select.sponge` — `crates/engine/src/shell_commands.rs`
- `tools.select.spotHealing` — `crates/engine/src/shell_commands.rs`
- `tools.select.triangle` — `crates/engine/src/shell_commands.rs`
- `tools.select.type` — `crates/engine/src/shell_commands.rs`
- `tools.select.verticalType` — `crates/engine/src/shell_commands.rs`
- `tools.select.verticalTypeMask` — `crates/engine/src/shell_commands.rs`
- `tools.select.zoom` — `crates/engine/src/shell_commands.rs`
- `type.attachToPath` — `crates/engine/src/cmd/type_extra_cmds.rs`
- `type.detachFromPath` — `crates/engine/src/cmd/type_extra_cmds.rs`
- `type.fillPath` — `crates/engine/src/cmd/type_area_cmds.rs`
- `type.font.favorite` — `crates/engine/src/cmd/font_library_cmds.rs`
- `type.font.library` — `crates/engine/src/cmd/font_library_cmds.rs`
- `ui.dock` — `crates/engine/src/shell_commands.rs`
- `updates.status` — `crates/engine/src/cmd/updates_cmds.rs`
- `view.colorSamplers.add` — `crates/engine/src/cmd/sampler_cmds.rs`
- `view.colorSamplers.clear` — `crates/engine/src/cmd/sampler_cmds.rs`
- `view.colorSamplers.delete` — `crates/engine/src/cmd/sampler_cmds.rs`
- `view.colorSamplers.list` — `crates/engine/src/cmd/sampler_cmds.rs`
- `view.colorSamplers.move` — `crates/engine/src/cmd/sampler_cmds.rs`
- `view.darkenInterface` — `crates/engine/src/shell_commands.rs`
- `view.editGuide` — `crates/engine/src/cmd/edit_cmds.rs`
- `view.lightenInterface` — `crates/engine/src/shell_commands.rs`
- `view.resetRulerOrigin` — `crates/engine/src/cmd/edit_cmds.rs`
- `view.setRulerOrigin` — `crates/engine/src/cmd/edit_cmds.rs`
- `window.panel.closeGroup` — `crates/engine/src/shell_commands.rs`
- `window.panel.closeOthers` — `crates/engine/src/shell_commands.rs`
- `window.panel.iconic` — `crates/engine/src/shell_commands.rs`
- `window.panel.minimize` — `crates/engine/src/shell_commands.rs`
- `window.panel.window` — `crates/engine/src/shell_commands.rs`
- `window.theme.softDark` — `crates/engine/src/shell_commands.rs`
- `window.theme.softLight` — `crates/engine/src/shell_commands.rs`
- `window.toggle.brushes` — `crates/engine/src/shell_commands.rs`


## Assumptions I could not confirm

- Native/windowed visual behavior and the embedding controller's Home/menu/panel composition were not run on a display.
- The in-browser request pump and tab throttling need runtime browser verification; wasm compilation checks their API compatibility only.
- No release wasm size measured because no release page was built.
- The adjacent private craftrules checkout is absent on this host; repository AGENTS.md and the controller's binding rules supplied the applicable requirements.
