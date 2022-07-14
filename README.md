# Darwinia Runtime Module Library
The Darwinia Runtime Module Library (DRML) is a darwinia.network maintained collection of Substrate runtime modules.

## Runtime Modules Overview
- [darwinia-balances](./frame/balances)
	- Provides functionality of handling balances.
- [darwinia-bridge-ethereum](./frame/bridge/ethereum/relay)
	- Pallet of the Ethereum > Darwinia relay.
- [darwinia-relay-authorities](./frame/bridge/relay-authorities)
	- Pallet that manages the relayer authorities.
- [darwinia-relayer-game](./frame/bridge/relayer-game)
	- Implementation of the Darwinia-Relayer-Game protocol.
- [darwinia-ethereum](./frame/dvm/ethereum/)
	- Ethereum block handling pallet of the EVM-compatible DVM system.
- [darwinia-evm](./frame/dvm/evm)
	- EVM execution handling pallet of the EVM-compatible DVM system.
- [darwinia-header-mmr](./frame/header-mmr)
	- Pallet that maintains the MMR(Merkle Mountain Range) data structure of the source chain headers.
- [darwinia-staking](./frame/staking)
	- Pallet that provides the staking-related features, nominating, validating etc.
- [darwinia-support](./frame/support)
	- Basic utility module.
- [from-substrate-issuing](./frame/wormhole/issuing/s2s)
	- Pallet of issuing assets on the Substrate base network.
- [to-ethereum-backing](./frame/wormhole/backing/ethereum)
	- Pallet of backing assets on the Ethereum network.
- [to-substrate-backing](./frame/wormhole/backing/s2s)
	- Pallet of backing assets on the Substrate base network.
- [to-tron-backing](./frame/wormhole/backing/tron)
	- Pallet of backing assets on the Tron network.

## Development
The darwinia-common has some test chains. you can start use [deploy.sh](tests/local-testnet/deploy.sh)

### Pangolin Testnet
#### With Script
```sh
./tests/local-testnet/deploy.sh pangolin
```

| validator | rpc-port | ws-port | node-key                                                      |
| --------- | -------- | ------- | ------------------------------------------------------------- |
| alice     | 30433    | 10044   | 0000000000000000000000000000000000000000000000000000000000101 |
| bob       | 30434    | 10045   | 0000000000000000000000000000000000000000000000000000000000102 |
| charlie   | 30435    | 10046   | 0000000000000000000000000000000000000000000000000000000000103 |
| dave      | 30436    | 10047   | 0000000000000000000000000000000000000000000000000000000000104 |
| eve       | 30437    | 10048   | 0000000000000000000000000000000000000000000000000000000000105 |
| ferdie    | 30438    | 10049   | 0000000000000000000000000000000000000000000000000000000000106 |

#### Manually
```sh
cargo build --release

target/release/drml \
	--chain pangolin-dev \
	--alice \
	--base-path tests/local-testnet/alice
```

### Pangoro Testnet
#### With Script
```sh
./tests/local-testnet/deploy.sh pangoro
```

| validator | rpc-port | ws-port | node-key                                                      |
| --------- | -------- | ------- | ------------------------------------------------------------- |
| alice     | 30533    | 10144   | 0000000000000000000000000000000000000000000000000000000000201 |
| bob       | 30534    | 10145   | 0000000000000000000000000000000000000000000000000000000000202 |
| charlie   | 30535    | 10146   | 0000000000000000000000000000000000000000000000000000000000203 |
| dave      | 30536    | 10147   | 0000000000000000000000000000000000000000000000000000000000204 |
| eve       | 30537    | 10148   | 0000000000000000000000000000000000000000000000000000000000205 |
| ferdie    | 30538    | 10149   | 0000000000000000000000000000000000000000000000000000000000206 |

#### Manually
```sh
cargo build --release

target/release/drml \
	--chain pangoro-dev \
	--alice \
	--base-path tests/local-testnet/alice
```

## Build
### NixOS
```sh
nix-shell
cargo build [-p drml] [--release] [--features template]
```
