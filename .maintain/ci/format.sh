#!/usr/bin/env bash

set -ex

rustup default stable
rustup component add rustfmt

source ~/.cargo/env

rustup --version
cargo --version
rustc --version

cargo clean

cargo fmt ${CI_PACKAGE/#/-p darwinia-}
