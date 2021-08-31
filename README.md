# Darwinia Runtime Module Library
The Darwinia Runtime Module Library (DRML) is a darwinia.network maintained collection of Substrate runtime modules.

## Runtime Modules Overview
- [darwinia-balances](./frame/balances)
	- Provides functionality of handling accounts and their balances.
- [darwinia-crab-backing](./frame/bridge/crab/backing)
	- Module of backing assets on the Crab network.
- [darwinia-crab-issuing](./frame/bridge/crab/issuing)
	- Module of issuing assets on the Crab network.
- [darwinia-ethereum-backing](./frame/bridge/ethereum/backing)
	- Module of backing assets on the Ethereum network.
- [darwinia-ethereum-issuing](./frame/bridge/ethereum/issuing)
	- Module of issuing assets on the Ethereum network.
- [darwinia-ethereum-relay](./frame/bridge/ethereum/relay)
	- Module of the Ethereum>Darwinia relayer.
- [darwinia-bridge-bsc](./frame/bridge/ethereum-bsc)
	- Module that verifies bsc(Binance Smart Chain) headers and authority set finality.
- [darwinia-relay-authorities](./frame/bridge/relay-authorities)
	- Module that manages the relayer authorities.
- [darwinia-relayer-game](./frame/bridge/relayer-game)
	- Implementation of the Darwinia Relayer Game Protocol.
- [darwinia-s2s-backing](./frame/bridge/s2s/backing)
	- Module that manages assets backing in Substrate-to-Substrate bridges.
- [darwinia-s2s-issuing](./frame/bridge/s2s/issuing)
	- Module that manages assets issuing in Substrate-to-Substrate bridges.
- [darwinia-tron-backing](./frame/bridge/tron/backing)
	- Module of backing assets on the Tron network.
- [darwinia-claims](./frame/claims)
	- Module to process claims from Ethereum addresses.
- [darwinia-democracy](./frame/democracy)
	- Module that handles the administration of general stakeholder voting..
- [darwinia-dvm](./frame/dvm)
	- Ethereum block handling module of the EVM-compatible DVM sytem.
- [darwinia-evm](./frame/evm)
	- EVM execution handling module of the EVM-compatible DVM sytem.
- [darwinia-dvm-dynamic-fee](./frame/dvm-dynamic-fee)
	- Extending fee handling module of the EVM-compatible DVM sytem.
- [darwinia-elections-phragmen](./frame/elections-phragmen)
	- An election module based on sequential phragmen.
- [darwinia-header-mmr](./frame/header-mmr)
	- Module that maintains the MMR(Merkle Mountain Range) data structure of the source chain headers.
- [darwinia-staking](./frame/staking)
	- Module that provides the staking-related features, nominating, validating etc.
- [darwinia-support](./frame/support)
	- Basic utility module.
- [darwinia-vesting](./frame/vesting)
	- Module that provides vesting protection of the blocked balance on an account.

## Development

### Deploy A Pangolin Local Testnet
```sh
tests/pangolin-local-testnet/deploy.sh
```
