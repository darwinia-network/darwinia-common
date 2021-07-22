
#!/bin/bash

RUST_LOG=darwinia_evm=debug ./target/release/drml benchmark \
  --chain dev \
  --wasm-execution compiled \
  --pallet darwinia_evm \
  --execution wasm \
  --extrinsic=* \
  --steps 100 \
  --repeat 10 \
  --raw \
  --heap-pages=4096 \
  --output=./frame/bridge/s2s/issuing/src/weight.rs \
  --template=./.maintain/frame-weight-template.hbs
