#!/bin/bash

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
REPO_PATH="$( cd "$( dirname "$0" )" && cd ../../ && pwd )"

CHAIN=$1
EXECUTION=$2

if [[ "$CHAIN" != "pangolin" ]] && [[ "$CHAIN" != "pangoro" ]] ; then
  echo "Missing chain name or not support chain, only supports [pangolin] or [pangoro]"
  exit 1
fi

if [ -z $EXECUTION ]; then
  EXECUTION=native
fi

LOG_DIR=$DIR/log
mkdir -p $LOG_DIR

DATA_DIR=$DIR/data
mkdir -p $DATA_DIR

EXECUTABLE=$REPO_PATH/target/release/drml

echo "Build \`drml\`"
# cargo build --release

index=100

if [[ "$CHAIN" == "pangolin" ]] ; then
  index=100
fi

if [[ "$CHAIN" == "pangoro" ]] ; then
  index=200
fi

for validator in alice bob charlie dave
do
  echo "Purge $validator's \`db\`, \`network\`, \`dvm\`"
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/db
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/network
  rm -rf $DATA_DIR/$validator/chains/$CHAIN/dvm

  echo "Firing $CHAIN Node $validator"
  $EXECUTABLE \
    --rpc-port $((9933 + index)) \
    --ws-port $((9944 + index)) \
    --port $((30333 + index)) \
    --unsafe-rpc-external \
    --unsafe-ws-external \
    --rpc-methods unsafe \
    --rpc-cors all \
    --execution $EXECUTION \
    --chain $CHAIN-local \
    -d $DATA_DIR/$validator \
    --$validator \
    &> $LOG_DIR/$validator.log &

  index=$((index + 1))
done
