#!/bin/sh
# cross clean  --target riscv64gc-unknown-linux-musl --release

cross build --no-default-features --features "slint/backend-linuxkms-noseat,slint/renderer-software,slint/compat-1-2" --target riscv64gc-unknown-linux-musl --release

