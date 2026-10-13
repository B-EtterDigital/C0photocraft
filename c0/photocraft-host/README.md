# c0/photocraft-host

The page C0VIBE loads to host PhotoCraft as a wrapped module (C0photocraft). It is ours; upstream
PhotoCraft does not ship it.

- Builds the unchanged `photocraft-web` app with the `heif` and `page-control` features. `page-control`
  (`apps/photocraft-web/src/page_control.rs` and `page_control/bridge.rs`) exposes the desktop control
  protocol to the page as `window.__photocraftControl`; this page aliases it as `window.__c0hostRequest`
  for the host.
- The host switches PhotoCraft's own bars off over that protocol (`ui.set`), draws the menus from
  `ui.menu.list` and mirrors the layer list from `document.inspect`.
- The host sets its own colours and radii with `ui.set {themeTokens}` (below).
- No ArtCraft name or logo. The page is transparent until the app paints.

## When a request is answered

A request does not wait for the display. The app handles control requests at the start of a frame, and
a browser gives a window that is not being presented about one frame a second, so the bridge runs one
egui pass of its own inside the call (the queue is drained, the interface laid out, nothing painted) and
the reply is ready when the call returns.

- A request that only reads (`ui.inspect`, `ui.menu.list`, `engine.commands`, and `engine.execute` of
  `document.inspect`, `session.inspect` or `document.pixel`) asks for no frame.
- Any other request asks for one frame afterwards so the change is seen.
- `ui.click`, `ui.key` and the other synthetic input, and a job the caller waits for, need several
  passes; they get one every 16 ms while they wait.
- `ui.screenshot` reads painted pixels and still waits for painted frames: about a second each in a
  window that is not presented.
- Before the app has started the request waits for its first frame, as it always did.

The facts this rests on are tested natively in `crates/ui-egui/tests/c0_control_passes.rs`. The browser
half (`page_control/bridge.rs`) is compiled for the web build but no test runs it.

## The toolbar as a palette: `panels.toolbar_floating`

    {"method": "ui.set", "params": {"panels": {"toolbar_floating": true}}}

A third toolbar mode after one and two columns (`toolbar_double`): the same tools in a palette drawn over
the canvas, which then takes the full width. The palette has a grip to drag it by and a pin that locks
it; it stays inside the canvas area. `ui.inspect().panels` reports `toolbar_floating` and
`toolbar_float: {"pos": [x, y] | null, "pinned": bool}`; `pos` is the palette's corner in points from
the canvas area's corner (`null`: the default corner). `ui.set {panels: {toolbar_float: {...}}}` replaces
the place as a whole. In the palette, the grip's context menu pins and docks; in the Pro themes the
header chevron's context menu lists the three modes (its click still switches the columns). The Studio
themes draw no chevron, so there the host owns the switch. Code: `crates/ui-egui/src/panels/toolbar_float.rs`;
tests: `crates/ui-egui/tests/c0_toolbar_floating.rs`.

## What the host relies on

`crates/ui-egui/tests/c0_host_contract.rs` fails when a pull changes a panel switch, a theme token, an
`ui.inspect` field, a command id or a menu entry the host uses. Run it after every pull.

## The host's palette: `ui.set {themeTokens}`

    {"method": "ui.set", "params": {"theme": "studioLight", "themeTokens": {"card": "#fcfcfd", "radius": 8}}}

`themeTokens` is read by the page bridge (`page_control/bridge.rs`), not by PhotoCraft's own `ui.set`; the
desktop socket does not know it. It replaces the whole set: tokens left out keep the preset's value, and
`null` or `{}` clears it. Colours are `"#rrggbb"` or `"#rrggbbaa"` (sRGB hex, alpha not premultiplied),
radii are numbers from 0 to 255 in points. The set stays in force when the preset changes, when a
document opens and on every frame (`crates/ui-egui/src/theme/host.rs`, applied inside `theme::apply`),
and egui's own widget visuals are derived from it. Pick the preset first: it still decides light or dark
defaults for everything not listed, and the layout (`pro` has tab strips, `studio*` has cards).

Reply: `{"ok": true, "result": {"themeTokens": {"applied": [...], "unknown": [...], "invalid": [...]}}}`.
`unknown` lists keys that are not tokens (a token renamed upstream shows up here after a pull);
`invalid` lists tokens whose value is not a colour or radius. Both are left out and the rest applies.
If the rest of the request fails (`{"ok": false, "error": ...}`), the palette is put back as it was.
`themeTokens` of any other JSON type fails the request and changes nothing.

Tokens (names are `Tokens` fields in `crates/ui-egui/src/theme.rs`):

| Token | Colours |
|---|---|
| `chrome` | window chrome: title bar, toolbars, panel fill |
| `canvas`, `canvas_dot` | the surround of the document, and the dots of its grid pattern |
| `dock` | the dock background behind the panels |
| `card`, `card_border` | panels, windows and dialogs, and their outline |
| `field`, `field_border` | inputs, dropdowns and resting buttons, and their outline |
| `hover`, `pressed` | a control under the pointer, and while pressed |
| `text`, `text_dim`, `text_faint` | body text, secondary labels, faint hints |
| `icon` | icon glyphs |
| `accent`, `accent_soft`, `accent_border`, `accent_text` | links, selection and active marks; the soft selection fill; the outline of a pressed or selected control; text on a selection |
| `separator` | divider lines |
| `shadow` | window and popup shadows (give it alpha) |
| `primary_bg`, `primary_text` | the default (primary) button and its label |
| `danger`, `warning` | error and warning text and marks |
| `tab_strip` | the panel tab strip (Pro layout) |
| `row_selected` | the selected row in lists (layers, history) |
| `caption_close`, `caption_close_text` | the title bar's Close button while hovered, and its glyph |
| `scrim` | the dim layer behind a modal (give it alpha) |
| `radius_sm`, `radius`, `radius_lg` | corner radius of controls, of menus and cards, of windows |

Not settable (reported as `unknown`): `kind`, `bevel`, `pro`, `round_chips`, `histogram_bg`,
`histogram_level`; the layer label colours and the scope hues are fixed in every theme.

Build, on a fleet host only (C0X `scripts/sow/upstream/toolchain.json` pins the tools):

    cd c0/photocraft-host && trunk build --release

Output: `dist/c0host/` (index.html, one .js, one .wasm). C0X serves it as `sow-app://c0photocraft/`.

## Branches

`main` mirrors upstream and holds nothing of ours. Our work is on `c0/main`; upstream is merged into it,
never rebased. Every edit inside an upstream file is listed in `PATCH-DEBT.md`.
