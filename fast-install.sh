#!/bin/sh

set -ex

function install {
    cargo build -p $1 && cp target/debug/$2$1 ~/.cargo/bin/$2$1
}

install "ginit" "cargo-"
install "ginit-android"
install "ginit-brainium"
install "ginit-ios"

