#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

BRANCH_NAME="${1:?}"
DEV_BRANCH="${2:?}"

if [[ $BRANCH_NAME == $DEV_BRANCH ]]; then
    cargo install --path ./ --force
    cargo ginit install-deps
fi
