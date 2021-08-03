#!/bin/bash

. ./prelude.sh

cargo build --release
cp ${NODE_PATH}/target/release/drml ../bin
