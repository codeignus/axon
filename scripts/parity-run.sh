#!/usr/bin/env bash
# Compare `axon check` vs migration driver `check` on listed fixtures.
# Phase 0 of docs/superpowers/plans/2026-04-30-axon-complete-migration.md
#
# Env:
#   AXON_BIN              path to repo compiler (default: ./target/build/axon/axon)
#   AXON_NATIVE_BUILD_BIN path to axon-native-build driver (default: try target/native-build-driver/debug/axon-native-build)
#   PARITY_FIXTURE_LIST   override fixture list file
#   PARITY_STRICT         if 1, require identical stdout+stderr (default: 0 = exit code only)
#   PARITY_SKIP_DRIVER    if 1, only run repo axon (default: 0)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FIXTURE_LIST="${PARITY_FIXTURE_LIST:-$ROOT/scripts/parity-fixture-list.txt}"
AXON_BIN="${AXON_BIN:-$ROOT/target/build/axon/axon}"
DRIVER_DEFAULT="$ROOT/target/native-build-driver/debug/axon-native-build"
DRIVER_BIN="${AXON_NATIVE_BUILD_BIN:-}"
if [[ -z "$DRIVER_BIN" && -x "$DRIVER_DEFAULT" ]]; then
  DRIVER_BIN="$DRIVER_DEFAULT"
fi
PARITY_STRICT="${PARITY_STRICT:-0}"
PARITY_SKIP_DRIVER="${PARITY_SKIP_DRIVER:-0}"

if [[ ! -f "$FIXTURE_LIST" ]]; then
  echo "error: fixture list not found: $FIXTURE_LIST" >&2
  exit 1
fi
if [[ ! -x "$AXON_BIN" ]]; then
  echo "error: AXON_BIN not executable: $AXON_BIN (build the compiler first)" >&2
  exit 1
fi

if [[ "$PARITY_SKIP_DRIVER" != "1" ]]; then
  if [[ -z "$DRIVER_BIN" || ! -x "$DRIVER_BIN" ]]; then
    echo "note: skipping driver parity (set AXON_NATIVE_BUILD_BIN or build axon-native-build)" >&2
    PARITY_SKIP_DRIVER=1
  fi
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

failures=0
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ -z "$line" || "$line" =~ ^# ]] && continue
  name="$(echo "$line" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ -z "$name" ]] && continue
  fix="$ROOT/tests/axon-cli/fixtures/$name"
  if [[ ! -d "$fix" ]]; then
    echo "SKIP: missing fixture dir: $fix" >&2
    continue
  fi
  if [[ ! -f "$fix/build.ax" ]]; then
    echo "SKIP: $name (no build.ax)" >&2
    continue
  fi

  echo "== parity: $name =="

  set +e
  (cd "$fix" && "$AXON_BIN" check "" >"$tmpdir/axon_out" 2>"$tmpdir/axon_err")
  axon_ec=$?
  set -e

  if [[ "$PARITY_SKIP_DRIVER" == "1" ]]; then
    echo "   axon: exit=$axon_ec"
    continue
  fi

  set +e
  (cd "$fix" && "$DRIVER_BIN" check >"$tmpdir/drv_out" 2>"$tmpdir/drv_err")
  drv_ec=$?
  set -e

  if [[ "$axon_ec" != "$drv_ec" ]]; then
    echo "   MISMATCH exit: axon=$axon_ec driver=$drv_ec" >&2
    failures=$((failures + 1))
  else
    echo "   exit: $axon_ec (match)"
  fi

  if [[ "$PARITY_STRICT" == "1" ]]; then
    if ! diff -q "$tmpdir/axon_out" "$tmpdir/drv_out" >/dev/null 2>&1 \
      || ! diff -q "$tmpdir/axon_err" "$tmpdir/drv_err" >/dev/null 2>&1; then
      echo "   MISMATCH output (strict mode)" >&2
      failures=$((failures + 1))
    fi
  fi
done < "$FIXTURE_LIST"

if [[ "$failures" -gt 0 ]]; then
  echo "parity-run: $failures mismatch(es)" >&2
  exit 1
fi
echo "parity-run: ok"
