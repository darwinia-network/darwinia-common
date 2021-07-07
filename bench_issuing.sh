#!/bin/bash

RUST_LOG=darwinia_s2s_issuing=debug ./target/release/drml benchmark \
  --chain dev \
  --wasm-execution compiled \
  --pallet darwinia_s2s_issuing \
  --execution native \
  --extrinsic=* \
  --steps 100 \
  --repeat 10 \
  --raw \
  --heap-pages=4096 \
  --output=./frame/bridge/s2s/issuing/src/weight.rs \
  --template=./.maintain/frame-weight-template.hbs
