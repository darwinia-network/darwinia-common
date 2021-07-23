#!/bin/bash

RUST_LOG=darwinia_evm=debug ./target/release/drml benchmark \
  --chain dev \
  --pallet darwinia_evm \
  --execution wasm \
  --wasm-execution compiled \
  --extrinsic=runner_execute \
  --steps 5 \
  --repeat 2 \
  --raw \
  --heap-pages=4096 \
  --output=./frame/evm/src/weight.rs \
  --template=./.maintain/frame-weight-template.hbs

