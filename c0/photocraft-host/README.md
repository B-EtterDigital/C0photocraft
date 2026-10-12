# c0/photocraft-host

The page C0VIBE loads to host PhotoCraft as a wrapped module (C0photocraft). It is ours; upstream
PhotoCraft does not ship it.

- Builds the unchanged `photocraft-web` app with the `heif` and `page-control` features. `page-control`
  (`apps/photocraft-web/src/page_control.rs`) exposes the desktop control protocol to the page as
  `window.__photocraftControl`; this page aliases it as `window.__c0hostRequest` for the host.
- The host switches PhotoCraft's own bars off over that protocol (`ui.set`), draws the menus from
  `ui.menu.list` and mirrors the layer list from `document.inspect`.
- No ArtCraft name or logo. The page is transparent until the app paints.

Build, on a fleet host only (C0X `scripts/sow/upstream/toolchain.json` pins the tools):

    cd c0/photocraft-host && trunk build --release

Output: `dist/c0host/` (index.html, one .js, one .wasm). C0X serves it as `sow-app://c0photocraft/`.

## Branches

`main` mirrors upstream and holds nothing of ours. Our work is on `c0/main`; upstream is merged into it,
never rebased. Every edit inside an upstream file is listed in `PATCH-DEBT.md`.
