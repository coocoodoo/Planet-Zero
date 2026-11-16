#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
HOST=$(rustc -vV | sed -n 's/^host: //p')
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$(rustc --print sysroot)/lib/rustlib/$HOST/bin/rust-lld"
export RUSTFLAGS="-C link-self-contained=yes -C target-cpu=cortex-a53 -C strip=symbols"
cd "$ROOT"
cargo build --locked --release --no-default-features --features cardputer --bin rust-fly-cardputer --target aarch64-unknown-linux-musl
mkdir -p dist
cp target/aarch64-unknown-linux-musl/release/rust-fly-cardputer dist/rust-fly-cardputer
