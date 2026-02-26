#!/bin/bash

rm -rf build 2&>/dev/null
mkdir build

# build windows
cargo build --target x86_64-pc-windows-gnu --release
# build linux x86_64
cargo build --target x86_64-unknown-linux-gnu --release

cp target/x86_64-pc-windows-gnu/release/lan-clipboard-sync.exe build/lan-clipboard-sync-windows-x86_64.exe
cp target/x86_64-unknown-linux-gnu/release/lan-clipboard-sync build/lan-clipboard-sync-linux-x86_64
