#!/bin/bash

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
REPO_PATH="$( cd "$( dirname "$0" )" && cd ../../ && pwd )"

CHAIN=$1

if [[ "$CHAIN" != "pangolin" ]] && [[ "$CHAIN" != "pangoro" ]] ; then
  echo "Missing chain name or not support chain, only supports [pangolin] or [pangoro]"
  exit 1
fi

LOG_DIR=$DIR/log
mkdir -p $LOG_DIR

DATA_DIR=$DIR/data
mkdir -p $DATA_DIR

EXECUTABLE=$REPO_PATH/target/release/drml

echo "Build \`drml\`"
cargo build --release


index=100

if [[ "$CHAIN" == "pangolin" ]] ; then
  index=100
fi

if [[ "$CHAIN" == "pangoro" ]] ; then
  index=200
fi

for validator in alice bob charlie dave eve ferdie
do
  echo "Purge $validator's \`db\`, \`network\`, \`dvm\`"
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/db
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/network
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/dvm

  echo "Firing ${CHAIN} Node ${validator}"
  ${EXECUTABLE} \
    --base-path $DATA_DIR/$validator \
    --$validator \
    --chain $CHAIN-local \
    --port $((303 + index)) \
    --ws-port $((994 + index)) \
    --node-key 0000000000000000000000000000000000000000000000000000000000000$((1 + index)) \
    --unsafe-ws-external \
    --rpc-cors all &> $LOG_DIR/$validator.log &

  index=$((index + 1))
done
