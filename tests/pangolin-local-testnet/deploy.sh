#!/bin/sh

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
REPO_PATH="$( cd "$( dirname "$0" )" && cd ../../ && pwd )"

LOG_DIR=$DIR/log
mkdir -p $LOG_DIR

DATA_DIR=$DIR/data
mkdir -p DATA_DIR

EXECUTABLE=$REPO_PATH/target/release/drml

echo "Build \`drml\`"
cargo build --release

index=0
for validator in alice bob charlie dave eve ferdie
do
  echo "Purge $validator's \`db\`, \`network\`, \`dvm\`"
  rm -rf $DATA_DIR/$validator/chains/pangolin/db
  rm -rf $DATA_DIR/$validator/chains/pangolin/network
  rm -rf $DATA_DIR/$validator/chains/pangolin/dvm

  echo "Firing Pangolin Node ${validator}"
  ${EXECUTABLE} \
    --base-path $DATA_DIR/pangolin-$validator \
    --$validator \
    --chain pangolin-local \
    --port $((30333 + index)) \
    --ws-port $((9944 + index)) \
    --node-key 000000000000000000000000000000000000000000000000000000000000000$((1 + index)) \
    --unsafe-ws-external \
    --rpc-cors all &> $LOG_DIR/$validator.log &

  index=$((index + 1))
done
