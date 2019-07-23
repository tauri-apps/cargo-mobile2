#!/usr/bin/env bash

set -ex

ci-tools/unlock_keychain.sh

export RUST_BACKTRACE=1

cargo test
