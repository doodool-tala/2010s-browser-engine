#!/usr/bin/env bash
# G1: universal hygiene gate. NEVER edit this file to make it pass.
# If it fails, fix the code. See docs/GATES.md.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ ! -f crates/nbe-core/Cargo.toml ]; then
  echo "G1 SKIP: workspace not bootstrapped on this branch"
  exit 0
fi

echo "== G1.1 format =="
cargo fmt --check

echo "== G1.2 clippy =="
cargo clippy --all-targets -- -D warnings

echo "== G1.3 tests =="
cargo test --all

echo "== G1.4 release build =="
cargo build --release --all

echo "== G1.5 forbidden-dependency check =="
if grep -nE 'nbe-(dom|layout|paint|js|text|parse-html|parse-css|bindings|webapi|renderer)' \
    crates/nbe-shell/Cargo.toml; then
  echo "G1 FAIL: nbe-shell must not link renderer-internal crates"
  exit 1
fi

echo "== G1.6 unsafe tripwire =="
if grep -rn "unsafe" crates --include="*.rs" | grep -v "//"; then
  echo "G1 FAIL: unsafe code found"
  exit 1
fi

echo "== G1.7 log determinism =="
A=$(cargo run -q -p nbe-shell 2>&1 >/dev/null)
B=$(cargo run -q -p nbe-shell 2>&1 >/decl)
if [ "$A" != "$B" ]; then
  echo "G1 FAIL: log output not deterministic"
  exit 1
fi

echo "G1 PASS"
