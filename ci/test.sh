#!/usr/bin/env bash

set -ex

export RUST_BACKTRACE=1

cargo test
