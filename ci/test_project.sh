#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

BRANCH_NAME="${1:?}"
DEV_BRANCH="${2:?}"

EXE_PATH_MOBILE="cargo mobile"
EXE_PATH_ANDROID="cargo android"
EXE_PATH_APPLE="cargo apple"

if [ $BRANCH_NAME != $DEV_BRANCH ]; then
    cargo build
    EXE_PATH_MOBILE=$(realpath "target/debug/cargo-mobile")
    EXE_PATH_ANDROID=$(realpath "target/debug/cargo-android")
    EXE_PATH_APPLE=$(realpath "target/debug/cargo-apple")
fi

# test by building a temporary project
rm -rf ./tmp
mkdir -p tmp
cd ./tmp
$EXE_PATH_MOBILE init --non-interactive
cargo check
$EXE_PATH_ANDROID check aarch64
$EXE_PATH_APPLE check aarch64
# FIXME: enable this when xcode11 is on jenkins
# $EXE_PATH apple check x86_64
cd -
