#!/bin/bash

echo "Run test node"
# RUST_LOG=moonbeam_service,moonbeam_rpc_debug,moonbeam_rpc_trace,moonbeam_client_evm_tracing,runtime_common,moonbeam_evm_tracer=debug \
./target/release/drml \
    --dev --tmp \
    --execution native \
    --wasm-execution compiled \
    --state-cache-size 1 \
    --ethapi=debug,trace,txpool \
    --wasm-runtime-overrides=bear-tracing \
     > debug.log 2>&1
