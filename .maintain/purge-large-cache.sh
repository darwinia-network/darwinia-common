#!/bin/sh

cargo clean -p drml 2> /dev/null || true
cargo clean -p drml-service 2> /dev/null || true
cargo clean -p pangoro-runtime 2> /dev/null || true
cargo clean -p pangolin-runtime 2> /dev/null || true
rm -rf target/debug/wbuild 2> /dev/null || true
