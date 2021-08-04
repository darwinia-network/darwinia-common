#!/bin/bash

set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"
REPO_PATH="$( cd "$( dirname "$0" )" && cd ../../ && pwd )"

LOG_PATH=${DIR}/log
DATA_PATH=${DIR}/data
BIN_PATH=${DIR}/bin

echo "===> Build node"
cargo build --release
cp ${REPO_PATH}/target/release/drml ${BIN_PATH}/

echo "===> Clean old chain data"
for i in 1 2 3 4 5 6
do
  rm -rf ${DATA_PATH}/n${i}/chains/pangolin/db
  rm -rf ${DATA_PATH}/n${i}/chains/pangolin/dvm
  rm -rf ${DATA_PATH}/n${i}/chains/pangolin/network
done

echo "===> Setup all the validators"
echo "start n1 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n1 \
  --alice \
  --chain pangolin \
  --port 30333 \
  --ws-port 9944 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node1.log &

echo "start n2 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n2 \
  --chain pangolin \
  --port 30334 \
  --ws-port 9945 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000002 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node2.log &

echo "start n3 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n3 \
  --chain pangolin \
  --port 30335 \
  --ws-port 9946 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000003 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node3.log &

echo "start n4 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n4 \
  --chain pangolin \
  --port 30336 \
  --ws-port 9947 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000004 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node4.log &

echo "start n5 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n5 \
  --chain pangolin \
  --port 30337 \
  --ws-port 9948 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000005 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node5.log &

echo "start n6 ..."
${BIN_PATH}/drml \
  --base-path ${DATA_PATH}/n6 \
  --chain pangolin \
  --port 30338 \
  --ws-port 9949 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000006 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --unsafe-ws-external \
  --rpc-cors all \
  --reserved-only \
  --reserved-nodes \
          "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
          "/ip4/127.0.0.1/tcp/30334/p2p/12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD" \
          "/ip4/127.0.0.1/tcp/30335/p2p/12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x" \
          "/ip4/127.0.0.1/tcp/30336/p2p/12D3KooWSsChzF81YDUKpe9Uk5AHV5oqAaXAcWNSPYgoLauUk4st" \
          "/ip4/127.0.0.1/tcp/30337/p2p/12D3KooWSuTq6MG9gPt7qZqLFKkYrfxMewTZhj9nmRHJkPwzWDG2" \
          "/ip4/127.0.0.1/tcp/30338/p2p/12D3KooWMz5U7fR8mF5DNhZSSyFN8c19kU63xYopzDSNCzoFigYk" \
  --validator &> ${LOG_PATH}/node6.log &

echo "===> The pangolin network is running successfully"
