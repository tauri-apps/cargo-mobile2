#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

BRANCH_NAME="${1:?}"
DEV_BRANCH="${2:?}"

EXE_PATH="cargo ginit"

if [ $BRANCH_NAME != $DEV_BRANCH ]; then
    cargo build
    EXE_PATH=$(realpath "target/debug/cargo-ginit")
fi

# test by building a temporary project
rm -rf ./tmp
mkdir -p tmp
cd ./tmp
$EXE_PATH init
$EXE_PATH android toolchain-init
$EXE_PATH ios toolchain-init
cargo check
$EXE_PATH android check aarch64
$EXE_PATH ios check aarch64
# FIXME: enable this when xcode11 is on jenkins
# $EXE_PATH ios check x86_64
cd -
