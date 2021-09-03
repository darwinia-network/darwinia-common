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

**Pangolin**

```sh
./tests/local-testnet/deploy.sh pangolin
```


| validator | rpc-port | ws-port | node-key                                                      |
| --------- | -------- | ------- | ------------------------------------------------------------- |
| alice     | 303100   | 994100  | 0000000000000000000000000000000000000000000000000000000000101 |
| bob       | 303101   | 994101  | 0000000000000000000000000000000000000000000000000000000000102 |
| charlie   | 303102   | 994102  | 0000000000000000000000000000000000000000000000000000000000103 |
| dave      | 303103   | 994103  | 0000000000000000000000000000000000000000000000000000000000104 |
| eve       | 303104   | 994104  | 0000000000000000000000000000000000000000000000000000000000105 |
| ferdie    | 303105   | 994105  | 0000000000000000000000000000000000000000000000000000000000106 |

**Pangoro**

```sh
./tests/local-testnet/deploy.sh pangoro
```

| validator | rpc-port | ws-port | node-key                                                      |
| --------- | -------- | ------- | ------------------------------------------------------------- |
| alice     | 303200   | 994200  | 0000000000000000000000000000000000000000000000000000000000201 |
| bob       | 303201   | 994201  | 0000000000000000000000000000000000000000000000000000000000202 |
| charlie   | 303202   | 994202  | 0000000000000000000000000000000000000000000000000000000000203 |
| dave      | 303203   | 994203  | 0000000000000000000000000000000000000000000000000000000000204 |
| eve       | 303204   | 994204  | 0000000000000000000000000000000000000000000000000000000000205 |
| ferdie    | 303205   | 994205  | 0000000000000000000000000000000000000000000000000000000000206 |


Or you can do it manually

**Build**

Build darwinia-common first

```bash
cargo build --release
```

**Pangolin**

```bash
./target/release/drml \
  --chain pangolin-local \
  --base-path /path/to/data/pangolin \
  --port 9955
  --ws-port 9956 \
  --alice
```

**Pangoro**

```bash
./target/release/drml \
  --chain pangoro-local \
  --base-path /path/to/data/pangoro \
  --port 9965
  --ws-port 9966 \
  --alice
```


