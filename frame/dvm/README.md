# DVM

Darwinia Network provides smart contract solutions based on DVM(Darwinia Virtual Machine), which is compatible with the Ethereum virtual machine paradigm at the low level. Therefore, It allows you to run unmodified Ethereum dapps.

## Pallets

Those pallets are part of the DVM system:

- `darwinia-dvm`: Ethereum block handling.
- `darwinia-evm`: Evm execution handling.
- `dvm-dynamic-fee`: Extends the fee handling logic so that it can be changed within the runtime.

## EVM Pallet Precompiles

- `darwinia-evm-precompile-blake2`：BLAKE2 precompile.
- `darwinia-evm-precompile-bn128`： BN128 precompile.
- `darwinia-evm-precompile-curve25519`: CURVE25519 precompile.
- `darwinia-evm-precompile-dispatch`: Enable interoperability between EVM contracts and other Substrate runtime components.
- `darwinia-evm-precompile-ed25519`: ED25519 precompile.
- `darwinia-evm-precompile-modexp`: MODEXP precompile.
- `darwinia-evm-precompile-sha3fips`: Standard SHA3 precompile.
- `darwinia-evm-precompile-simple`: Four basic precompiles in Ethereum EVMs.
- `darwinia-evm-precompile-encoder`: Encode substrate dispatch call.
- `darwinia-evm-precompile-transfer`: Transfer asset from dvm account to substrate account.
