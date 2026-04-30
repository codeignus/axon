#!/usr/bin/env bash
# After moving bootstrap-compiler elsewhere: only `axon check` / `axon build` (no cargo in this script).
# Requires a runnable compiler binary and AXON_NATIVE_TOOLCHAIN pointing at that Cargo workspace root.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOL="${AXON_NATIVE_TOOLCHAIN:?export AXON_NATIVE_TOOLCHAIN=/path/to/bootstrap-compiler (Cargo workspace root)}"
BIN="${AXON_AXON_BIN:-$ROOT/target/build/axon/axon}"
if [[ ! -x "$BIN" ]]; then
  echo "error: missing executable $BIN (set AXON_AXON_BIN)" >&2
  exit 1
fi
cd "$ROOT"
export AXON_NATIVE_TOOLCHAIN="$TOOL"

echo "== axon check =="
"$BIN" check ""

echo "== axon build =="
"$BIN" build

echo "== axon check again =="
"$BIN" check ""

echo "== axon build again =="
"$BIN" build

echo "ok: independent axon-only verification complete"
