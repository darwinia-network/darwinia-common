echo "1. Cargo build release..."
cargo build --release --features evm-tracing

echo "2. Move out substitude runtime..."
# mkdir runtime-overrides
# cp target/release/wbuild/pangolin-runtime/pangolin_runtime.compact.compressed.wasm runtime-overrides

echo "3. Setup node..."
./target/release/drml \
    --dev \
    --tmp \
    --execution wasm \
    --ethapi-debug-targets=debug,trace \
    --wasm-runtime-overrides . \
    --alice