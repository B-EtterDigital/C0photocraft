#!/usr/bin/env bash
# The C0X payload builder runs a page's own build.sh when it has one (scripts/sow/upstream/build-payload.mjs).
# Size tuning for the guest page only (the 24 MiB cap in the C0X pin): smallest code, one unit, fat LTO, no debug info.
# Same recipe that took C0pdf from 46.4 MB to 30.4 MB on 2026-10-28; panic strategy stays upstream's.
set -euo pipefail
cd "$(dirname "$0")"
export CARGO_PROFILE_RELEASE_OPT_LEVEL=s  # s, not z: pixel work runs in wasm here, keep it fast
export CARGO_PROFILE_RELEASE_LTO=fat
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_STRIP=debuginfo
if [[ -n "${CRAFT_FONTS_DIR:-}" && ! -d "$CRAFT_FONTS_DIR" ]]; then unset CRAFT_FONTS_DIR; fi
exec smoac-slot run heavy 2 3600 -- env CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" NO_COLOR=true trunk build --release "$@"
