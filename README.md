# Darwinia Runtime Module Library
The Darwinia Runtime Module Library (DRML) is a darwinia.network maintained collection of Substrate runtime modules.

## Runtime Modules Overview
- [darwinia-balances](./frame/balances)
	- Desc.
- [darwinia-bridge-ethereum](./frame/bridge/ethereum/relay)
	- Desc.
- [darwinia-bridge-bsc](./frame/bridge/bsc)
	- Desc.
- [darwinia-relay-authorities](./frame/bridge/relay-authorities)
	- Desc.
- [darwinia-relayer-game](./frame/bridge/relayer-game)
	- Desc.
- [darwinia-claims](./frame/claims)
	- Desc.
- [darwinia-democracy](./frame/democracy)
	- Desc.
- [darwinia-dvm](./frame/dvm)
	- Desc.
- [darwinia-dvm-dynamic-fee](./frame/dvm-dynamic-fee)
	- Desc.
- [darwinia-elections-phragmen](./frame/elections-phragmen)
	- Desc.
- [darwinia-evm](./frame/evm)
	- Desc.
- [darwinia-header-mmr](./frame/header-mmr)
	- Desc.
- [darwinia-staking](./frame/staking)
	- Desc.
- [darwinia-support](./frame/support)
	- Desc.
- [darwinia-vesting](./frame/vesting)
	- Desc.
- [to-ethereum-backing](./frame/wormhole/backing/ethereum)
	- Desc.
- [from-ethereum-issuing](./frame/wormhole/issuing/ethereum)
	- Desc.
- [to-substrate-backing](./frame/wormhole/backing/s2s)
	- Desc.
- [from-substrate-issuing](./frame/wormhole/issuing/s2s)
	- Desc.
- [to-tron-backing](./frame/wormhole/backing/tron)
	- Desc.

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
