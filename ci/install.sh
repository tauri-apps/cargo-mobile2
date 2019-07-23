#!/usr/bin/env bash

set -ex

ci-tools/unlock_keychain.sh

export RUST_BACKTRACE=1

BRANCH_NAME="${1:?}"
DEV_BRANCH="${2:?}"

if [[ $BRANCH_NAME == $DEV_BRANCH ]]; then
    cargo install --path cargo-ginit/ --force
fi
