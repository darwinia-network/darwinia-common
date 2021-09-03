Usage
===


## Chain

The darwinia-common has some test chains, here is the instruction document.

All chain start use [drml](../node/cli) node binary, The first compile darwinia-common.

```shell
git clone https://github.com/darwinia-network/darwinia-common.git
cd darwinia-common
cargo build --release
```

### pangolin

```bash
./target/release/drml \
  --chain pangolin-dev \
  --base-path /path/to/data/pangolin \
  --ws-port 9956 \
  --alice
```

### pangoro

```bash
./target/release/drml \
  --chain pangoro-dev \
  --base-path /path/to/data/pangoro \
  --ws-port 9955 \
  --alice
```


