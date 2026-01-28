#!/usr/bin/env bash
set -euo pipefail

crate_name="tetanus-attack"

rustup target add wasm32-unknown-unknown

cargo build --release --target wasm32-unknown-unknown

wasm-bindgen \
  --target web \
  --out-dir web \
  "target/wasm32-unknown-unknown/release/${crate_name}.wasm"
