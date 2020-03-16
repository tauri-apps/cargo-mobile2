#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

cargo build
EXE_PATH=$(realpath "target/debug/cargo-ginit")

# test by building a temporary project
rm -rf ./tmp
mkdir -p tmp
cd ./tmp
$EXE_PATH init --non-interactive
cargo check
$EXE_PATH android check aarch64
$EXE_PATH ios check aarch64
# FIXME: enable this when xcode11 is on jenkins
# $EXE_PATH ios check x86_64
cd -
