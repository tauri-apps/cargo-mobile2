#!/bin/sh

# This script is only intended for use during development. It's faster than a
# regular `cargo install`, which makes iteration more pleasant.

set -ex

if ! [[ -z "${CARGO_HOME}" ]]; then
  cargoHome="${CARGO_HOME}"
else
  cargoHome="~/.cargo"
fi

if ! [[ -z "${CARGO_TARGET_DIR}" ]]; then
  cargoTargetDir="${CARGO_TARGET_DIR}"
else if ! [[ -z "${CARGO_BUILD_TARGET_DIR}" ]]; then
  cargoTargetDir="${CARGO_BUILD_TARGET_DIR}"
else
  cargoTargetDir="target"
fi

function copy {
    cp $cargoTargetDir/debug/cargo-$1 $cargoHome/bin/cargo-$1
}

cargo build -p cargo-mobile2 $@
copy "android"
copy "apple"
copy "mobile"

