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
After resolution: 54 nonblank added lines in 20 files, 14 replaced, 2 removed, 2 additional blanks;
indentation-only changes excluded. See PATCH-DEBT.md for each seam and moved-path accounting.
The 3-line delegation for filtered tiling moves its policy into c0_sessions.rs and preserves upstream's
stable tile positions. Debt remains below the listed baseline. All conflicted paths, including zero-debt
moved files and CI policy removals, are now accounted for in that ledger.

## Verification

Pending at merge checkpoint. First build is queued under `smoac-slot run gate 3 3600 --`;
the host landing reservation initially holds admission until 13:06 UTC.
All Cargo commands use `CARGO_BUILD_JOBS=4` and the requested shared target directory.
Initial pre-build disk: 81%; load averages 5.26 / 9.62 / 14.48.
No Rust crate or Cargo manifest exists under c0/; host tests live in ui-egui and photocraft-web.
Final pass counts, per-command load pairs and upstream-red attribution will be recorded here.
No release web page build requested or performed; release wasm size unmeasured, cap unchanged at 25165824 bytes.

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
- Panels: shared craft_ui docking replaces native docking code. Floating toolbar retains upstream tool buttons, RTL panel direction and the new chevron in every theme. Palette sizing still depends on toolbar spacing; native viewport interaction needs a visual follow-up beyond headless tests.
- Theme: egui 0.37 uses Role and Id::unique; our palette and harness IDs were updated. The canvas-dot texture cache key still matches our invalidation key. Runtime tokens resolve through upstream's parser and apply path.
- Community links: switch remains default-on and is omitted from workspace saves, as before. The separate cleanup removes workflows and docs/brand material except its license. The compiled titlebar icon now lives under assets/app-icon, not docs/brand; no blank replacement at docs/brand is required.
- Agent server: control dispatch moved to control/dispatch.rs; c0 methods stay ahead of upstream methods. eframe 0.37 still exposes WebRunner::add_event_listener and AppRunner::logic. New command rows include needsDocument/disabledReason/keywords. Native control authentication/path changes are upstream; the page bridge retains its own pass pump and request limits.

## New upstream commands and menu items

Type > Anti-Alias > None (`type.antiAlias.none`) is the one added static menu catalog row.
New/expanded implemented families include History snapshots/options and step commands, action editing,
ATN load/save, layer-style contours and ASL libraries, type path attachment/area conversion, color samplers,
font favourites/library filters, export presets, layer XMP, guide editing/ruler origins, printer discovery,
face detection and update checks. Some IDs existed as disabled catalog entries before acquiring implementations.
The source inventory below also includes newly explicit shell/tool aliases (not all are new visible tools).

New literal IDs absent from the baseline source (engine, shell and menu declarations). Existing menu entries that acquired implementations and generated command families are listed separately below.

- `actions.assignSet` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.conditional` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.dialog` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.insert` — `crates/engine/src/cmd/actions_edit_cmds.rs`
- `actions.setColor` — `crates/engine/src/cmd/actions_cmds.rs`
- `brush.mixer.clean` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.mixer.load` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.mixer.sample` — `crates/engine/src/cmd/brush_cmds.rs`
- `brush.presets.importImage` — `crates/engine/src/cmd/preset_import_cmds.rs`
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
- `history.redo` — `crates/engine/src/cmd/history_cmds.rs`
- `history.renameSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.selectSnapshot` — `crates/engine/src/cmd/history_cmds.rs`
- `history.stepBackward` — `crates/engine/src/cmd/history_menu_cmds.rs`
- `history.stepForward` — `crates/engine/src/cmd/history_menu_cmds.rs`
- `history.undo` — `crates/engine/src/cmd/history_cmds.rs`
- `image.detectFaces` — `crates/engine/src/cmd/face_cmds.rs`
- `layer.artboard.addBeside` — `crates/engine/src/cmd/artboard_cmds.rs`
- `layer.fill.pattern.set` — `crates/engine/src/cmd/atn_cmds/steps_make.rs`
- `layer.layerStyle` — `crates/engine/src/layer_style.rs`
- `layer.layerStyle.contours.delete` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.list` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.load` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.new` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.layerStyle.contours.save` — `crates/engine/src/cmd/contour_lib_cmds.rs`
- `layer.mergeGroup` — `crates/engine/src/cmd.rs`
- `layer.toggleLockTransparency` — `crates/engine/src/cmd/extra_cmds.rs`
- `layerComp.reorder` — `crates/engine/src/cmd/comps_cmds.rs`
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
- `tools.temporary.erase` — `crates/engine/src/prefs/shortcuts.rs`
- `type.attachToPath` — `crates/engine/src/cmd/type_extra_cmds.rs`
- `type.detachFromPath` — `crates/engine/src/cmd/type_extra_cmds.rs`
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
