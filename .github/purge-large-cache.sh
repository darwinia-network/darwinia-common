#!/bin/sh

cargo clean -p drml 2> /dev/null || true
cargo clean -p drml-node-service 2> /dev/null || true
cargo clean -p pangoro-runtime 2> /dev/null || true
cargo clean -p pangolin-runtime 2> /dev/null || true
cargo clean -p template-runtime 2> /dev/null || true
rm -rf target/debug/wbuild 2> /dev/null || true

cargo clean --release -p drml 2> /dev/null || true
cargo clean --release -p drml-node-service 2> /dev/null || true
cargo clean --release -p pangoro-runtime 2> /dev/null || true
cargo clean --release -p pangolin-runtime 2> /dev/null || true
cargo clean --release -p template-runtime 2> /dev/null || true
rm -rf target/release/wbuild 2> /dev/null || true
