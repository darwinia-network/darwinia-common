#!/usr/bin/env bash

set -ex

rustup default "$RUST_TOOLCHAIN"

source ~/.cargo/env

rustup --version
cargo --version
rustc --version

cargo test ${CI_PACKAGE/#/-p darwinia-}
