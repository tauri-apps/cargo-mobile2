#!/bin/sh

# This script is only intended for use during development. It's faster than a
# regular `cargo install`, which makes iteration more pleasant.

set -ex

function copy {
    cp target/debug/cargo-$1 ~/.cargo/bin/cargo-$1
}

cargo build -p cargo-mobile $@
copy "android"
copy "apple"
copy "mobile"

