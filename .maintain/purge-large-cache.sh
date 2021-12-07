#!/bin/sh

cargo clean --release -p drml 2> /dev/null || true
cargo clean --release -p drml-service 2> /dev/null || true
cargo clean --release -p pangoro-runtime 2> /dev/null || true
cargo clean --release -p pangolin-runtime 2> /dev/null || true
cargo clean --release -p template-runtime 2> /dev/null || true
rm -rf target/release/wbuild 2> /dev/null || true
