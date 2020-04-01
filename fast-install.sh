#!/bin/sh

set -ex

function copy {
    cp target/debug/cargo-$1 ~/.cargo/bin/cargo-$1
}

cargo build -p cargo-mobile
copy "android"
copy "apple"
copy "mobile"

