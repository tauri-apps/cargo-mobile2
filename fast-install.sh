#!/bin/sh

# This script is only intended for use during development. It's faster than a
# regular `cargo install`, which makes iteration more pleasant.

set -ex

function copy {
    cp target/debug/cargo-$1 ~/.cargo/bin/cargo-$1
}

# TODO: using this feature here makes this script less useful for anyone outside
# the company
cargo build -p cargo-mobile --features brainium
copy "android"
copy "apple"
copy "mobile"

