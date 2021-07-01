#!/bin/bash

./target/release/drml benchmark \
  --chain dev \
  --execution wasm \
  --wasm-execution compiled \
  --pallet darwinia_s2s_issuing \
  --extrinsic=* \
  --steps 100 \
  --repeat 10 \
  --raw \
  --heap-pages=4096 \
  --output=./frame/bridge/s2s/issuing/src/weight.rs \
  --template=./.maintain/frame-weight-template.hbs
