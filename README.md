# Darwinia Runtime Module Library
The Darwinia Runtime Module Library (DRML) is a darwinia.network maintained collection of Substrate runtime modules.

## Runtime Modules Overview
- [darwinia-balances](./frame/balances)
	- Provides functionality of handling accounts and their balances.
- [darwinia-bridge-ethereum](./frame/bridge/ethereum/relay)
	- Pallet of the Ethereum > Darwinia relayer.
- [darwinia-bridge-bsc](./frame/bridge/bsc)
	- Pallet that verifies BSC(Binance Smart Chain) headers and authority set finality.
- [darwinia-relay-authorities](./frame/bridge/relay-authorities)
	- Pallet that manages the relayer authorities.
- [darwinia-relayer-game](./frame/bridge/relayer-game)
	- Implementation of the Darwinia Relayer Game Protocol.
- [darwinia-claims](./frame/claims)
	- Pallet to process claims from Ethereum addresses.
- [darwinia-democracy](./frame/democracy)
	- Pallet that handles the administration of general stakeholder voting.
- [darwinia-dvm](./frame/dvm)
	- Ethereum block handling module of the EVM-compatible DVM system.
- [darwinia-evm](./frame/evm)
	- EVM execution handling module of the EVM-compatible DVM system.
- [darwinia-dvm-dynamic-fee](./frame/dvm-dynamic-fee)
	- Extending fee handling module of the EVM-compatible DVM system.
- [darwinia-elections-phragmen](./frame/elections-phragmen)
	- An election module based on sequential phragmen.
- [darwinia-header-mmr](./frame/header-mmr)
	- Pallet that maintains the MMR(Merkle Mountain Range) data structure of the source chain headers.
- [darwinia-staking](./frame/staking)
	- Pallet that provides the staking-related features, nominating, validating etc.
- [darwinia-support](./frame/support)
	- Basic utility module.
- [darwinia-vesting](./frame/vesting)
	- Pallet that provides vesting protection of the blocked balance on an account.
- [from-ethereum-issuing](./frame/wormhole/issuing/ethereum)
	- Pallet that manages assets issuing in Substrate-to-Substrate bridges.
- [from-substrate-issuing](./frame/wormhole/issuing/s2s)
	- Pallet of issuing assets on the Crab network.
- [to-ethereum-backing](./frame/wormhole/backing/ethereum)
	- Pallet of backing assets on the Ethereum network.
- [to-substrate-backing](./frame/wormhole/backing/s2s)
	- Pallet that manages assets backing in Substrate-to-Substrate bridges.
- [to-tron-backing](./frame/wormhole/backing/tron)
	- Pallet of backing assets on the Tron network.

## Development

### Deploy A Pangolin Local Testnet
```sh
tests/pangolin-local-testnet/deploy.sh
```
