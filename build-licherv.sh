#!/bin/sh
# Steps
# - Build the toolchain from https://github.com/scpcom/LicheeSG-Nano-Build/
#   - recommended: Use Debian 12, instructions for building the toolchain in the README.md
# - Install required debian packages:
#   - sudo apt install linux-libc-dev-riscv64-cross libclang-dev clang
# - Install rust tooling:
#   - rustup target add riscv64gc-unknown-linux-musl
# - Download and extract modernized linker binaries
#   - get riscv64-linux-musl-cross.tgz from https://musl.cc/#binaries
#   - extract to $HOME/riscv64-linux-musl-cross/bin
# - run build-licherv.sh
# - find build artifacts at: target/riscv64gc-unknown-linux-gnu/{debug,release}/resonance

# export SLINT_BACKEND=linuxkms-software
# export SLINT_BACKEND=winit-software


export TOOLCHAIN_PATH="$HOME/projects/scpcom/LicheeSG-Nano-Build"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$TOOLCHAIN_PATH/buildroot/output/cvitek_SG200X_musl_riscv64/host/riscv64-buildroot-linux-musl/sysroot"
export PKG_CONFIG_PATH="$PKG_CONFIG_SYSROOT_DIR/usr/lib/pkgconfig:$PKG_CONFIG_SYSROOT_DIR/usr/share/pkgconfig"
export CFLAGS="--sysroot=$PKG_CONFIG_SYSROOT_DIR"
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$PKG_CONFIG_SYSROOT_DIR"
#export PATH="$PATH:$TOOLCHAIN_PATH/host-tools/gcc/riscv64-linux-musl-x86_64/bin"

# Download riscv64-linux-musl-cross.tgz from https://musl.cc/#binaries and extract to $HOME/riscv64-linux-musl-cross/bin
export PATH="$PATH:$HOME/riscv64-linux-musl-cross/bin"
export SLINT_BACKEND_LINUXFB=1
SLINT_BACKEND_LINUXFB=1 cargo build --no-default-features --features "slint/backend-linuxkms-noseat,slint/renderer-software,slint/compat-1-2" --target riscv64gc-unknown-linux-musl --release
# SLINT_BACKEND_LINUXFB=1 cargo build --target riscv64gc-unknown-linux-musl --release
# cargo run --features "slint/feature1,slint/feature2"
