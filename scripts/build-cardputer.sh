#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
HOST=$(rustc -vV | sed -n 's/^host: //p')
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$(rustc --print sysroot)/lib/rustlib/$HOST/bin/rust-lld"
FLAGS=("-Clink-self-contained=yes" "-Ctarget-cpu=cortex-a53" "-Cstrip=symbols")
TARGET_DIR=${CARGO_TARGET_DIR:-target}
OUTPUT=dist/flight-club-cardputer
case "${1:-}" in
  "") ;;
  --pgo-generate)
    [[ $# == 1 ]] || { echo "用法：$0 --pgo-generate" >&2; exit 1; }
    FLAGS+=("-Cprofile-generate")
    TARGET_DIR=${CARGO_TARGET_DIR:-target/pgo-generate}
    OUTPUT=dist/flight-club-cardputer-pgo-generate
    ;;
  --pgo-use)
    [[ $# == 2 && -f "$2" ]] || { echo "请通过 --pgo-use 指定合并后的 .profdata 文件" >&2; exit 1; }
    PROFILE="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
    FLAGS+=("-Cprofile-use=$PROFILE" "-Cllvm-args=-pgo-warn-missing-function")
    TARGET_DIR=${CARGO_TARGET_DIR:-target/pgo-use}
    ;;
  *) echo "用法：$0 [--pgo-generate | --pgo-use FILE]" >&2; exit 1 ;;
esac
export CARGO_ENCODED_RUSTFLAGS="$(IFS=$'\x1f'; echo "${FLAGS[*]}")"
cd "$ROOT"
cargo build --locked --release --no-default-features --features cardputer --bin flight-club-cardputer --target aarch64-unknown-linux-musl --target-dir "$TARGET_DIR"
mkdir -p dist
cp "$TARGET_DIR/aarch64-unknown-linux-musl/release/flight-club-cardputer" "$OUTPUT"
