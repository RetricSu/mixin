#!/bin/sh
# 编出 dist/ 里给普通人用的成品：Mac 通用二进制 + Windows exe
set -e
cd "$(dirname "$0")"

rustup target add aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-gnu >/dev/null

cargo test
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-gnu

mkdir -p dist
lipo -create \
  target/aarch64-apple-darwin/release/mixin \
  target/x86_64-apple-darwin/release/mixin \
  -output dist/密信
cp target/x86_64-pc-windows-gnu/release/mixin.exe dist/密信.exe
chmod +x dist/密信 dist/双击我.command

echo
echo "编好了："
ls -lh dist/密信 dist/密信.exe
