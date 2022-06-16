# DVM

Darwinia Network provides smart contract solutions based on DVM(Darwinia Virtual Machine), which is compatible with the Ethereum virtual machine paradigm at a low level. Therefore, It allows you to run unmodified Ethereum DApps.

## Pallets

Those pallets are part of the DVM system:

- `darwinia-ethereum`: Ethereum block handling.
- `darwinia-evm`: EVM execution handling.

## EVM Pallet Customed Precompiles

- `darwinia-evm-precompile-bridge-ethereum`: The encoder precompile for ethereum bridge.
- `darwinia-evm-precompile-bridge-s2s`: The encoder precompile for s2s bridge.
- `darwinia-evm-precompile-bls12-381`: The BLS12381 precompile.
- `darwinia-evm-precompile-mpt`: The precompile to verify merkle patricia trie storage proof.
- `darwinia-evm-precompile-state-storage`: The precompile to read state storage with filter.
- `darwinia-evm-precompile-dispatch`: Enable interoperability between EVM contracts and other Substrate runtime components.
- `darwinia-evm-precompile-transfer`: Transfer asset from DVM account to substrate account.
