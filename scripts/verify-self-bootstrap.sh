#!/usr/bin/env bash
# Prove: bootstrap cargo produces axon → that binary runs `build` → same binary can `check` and `build` again.
# Each stage preserves a suffixed binary for later comparison.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MANIFEST="${AXON_BOOTSTRAP_MANIFEST:-}"
if [[ -z "$MANIFEST" ]]; then
  for c in \
    "$ROOT/bootstrap-compiler/Cargo.toml" \
    "$ROOT/depreciating-soon-compiler-do-not-rename/Cargo.toml"
  do
    if [[ -f "$c" ]]; then
      MANIFEST="$c"
      break
    fi
  done
fi
if [[ -z "${MANIFEST}" || ! -f "${MANIFEST}" ]]; then
  echo "error: no bootstrap Cargo.toml found; set AXON_BOOTSTRAP_MANIFEST=/path/to/Cargo.toml" >&2
  exit 1
fi

# Reference `axon-codegen` uses inkwell with LLVM 21. If `llvm-sys` cannot find it,
# set LLVM_SYS_211_PREFIX explicitly (or install llvm-21-dev / matching Nix package).
# When unset, probe common layouts so one machine can use distro LLVM 21 and
# another can use a custom prefix without editing the script.
if [[ -z "${LLVM_SYS_211_PREFIX:-}" ]]; then
  if command -v llvm-config-21 >/dev/null 2>&1; then
    export LLVM_SYS_211_PREFIX
    LLVM_SYS_211_PREFIX="$(llvm-config-21 --prefix)"
    echo "== hint: LLVM_SYS_211_PREFIX=$LLVM_SYS_211_PREFIX (from llvm-config-21) =="
  elif command -v llvm-config >/dev/null 2>&1; then
    _llvm_ver="$(llvm-config --version 2>/dev/null || true)"
    if [[ "${_llvm_ver}" == 21.* ]]; then
      export LLVM_SYS_211_PREFIX
      LLVM_SYS_211_PREFIX="$(llvm-config --prefix)"
      echo "== hint: LLVM_SYS_211_PREFIX=$LLVM_SYS_211_PREFIX (from llvm-config, version ${_llvm_ver}) =="
    fi
  fi
  if [[ -z "${LLVM_SYS_211_PREFIX:-}" && -d /usr/lib/llvm/21 ]]; then
    export LLVM_SYS_211_PREFIX=/usr/lib/llvm/21
    echo "== hint: LLVM_SYS_211_PREFIX=$LLVM_SYS_211_PREFIX (default Linux path) =="
  fi
fi

OUT_DIR="$ROOT/target/build/axon"
BIN="$OUT_DIR/axon"

mkdir -p "$OUT_DIR"

preserve_binary() {
  local suffix="$1"
  local src="$BIN"
  local dst="$OUT_DIR/axon_${suffix}"
  if [[ ! -x "$src" ]]; then
    echo "error: cannot preserve: $src is not executable" >&2
    exit 1
  fi
  cp -p "$src" "$dst"
  echo "== preserved: $dst =="
}

verify_binary() {
  local path="$1"
  local label="$2"
  if [[ ! -x "$path" ]]; then
    echo "error: $label binary not executable at $path" >&2
    exit 1
  fi
  "$path" check ""
  echo "== verified: $label binary is runnable =="
}

echo "== stage0: bootstrap (cargo) produces self compiler artifact =="
if ! cargo run --manifest-path "$MANIFEST" -p axon -- build; then
  echo "error: bootstrap build failed. If the failure mentions llvm-sys or LLVM," >&2
  echo "  install LLVM 21 dev libraries and export LLVM_SYS_211_PREFIX to the install prefix" >&2
  echo "  (e.g. export LLVM_SYS_211_PREFIX=\$(llvm-config-21 --prefix))." >&2
  exit 1
fi

if [[ ! -x "$BIN" ]]; then
  echo "error: expected executable at $BIN after bootstrap build" >&2
  exit 1
fi
preserve_binary "rustcompiled1"
verify_binary "$OUT_DIR/axon_rustcompiled1" "rustcompiled1"

echo "== stage1: rustcompiled1 runs check + build → selfcompiled1 =="
"$OUT_DIR/axon_rustcompiled1" check ""
"$OUT_DIR/axon_rustcompiled1" build
preserve_binary "selfcompiled1"
verify_binary "$OUT_DIR/axon_selfcompiled1" "selfcompiled1"

echo "== stage2: selfcompiled1 runs check + build → selfcompiled2 =="
"$OUT_DIR/axon_selfcompiled1" check ""
"$OUT_DIR/axon_selfcompiled1" build
preserve_binary "selfcompiled2"
verify_binary "$OUT_DIR/axon_selfcompiled2" "selfcompiled2"

echo "== stage3: selfcompiled2 runs check + build → selfcompiled3 =="
"$OUT_DIR/axon_selfcompiled2" check ""
"$OUT_DIR/axon_selfcompiled2" build
preserve_binary "selfcompiled3"
verify_binary "$OUT_DIR/axon_selfcompiled3" "selfcompiled3"

echo "ok: self-bootstrap verification complete (4 suffixed binaries preserved)"
