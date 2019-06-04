#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

# test by building a temporary project
rm -rf ./tmp
mkdir -p tmp
cd ./tmp
cargo ginit init
cargo ginit android toolchain-init
cargo ginit ios toolchain-init
cargo check
cargo ginit android check aarch64
cargo ginit ios check aarch64
cd -
