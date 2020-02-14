#!/bin/sh

set -ex

function install {
    cargo build -p $1 && cp target/debug/$2$1 ~/.cargo/bin/$2$1
}

if [ -z "$1" ]; then
    install "ginit" "cargo-"
    install "ginit-android"
    install "ginit-brainium"
    install "ginit-ios"
else
    if [ "$1" = "ginit" ]; then
        install "ginit" "cargo-"
    else
        install "ginit-$1"
    fi
fi

