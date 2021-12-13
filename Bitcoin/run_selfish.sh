#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
binary_path="./target/release/bitcoin"
$binary_path -vv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 --selfish true &> local_node_1.log &
pid1="$!"
$binary_path -vv --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000   &> local_node_2.log &
pid2="$!"
$binary_path -vv --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001   &> local_node_3.log &
pid3="$!"
$binary_path -vv --p2p 127.0.0.1:6003 --api 127.0.0.1:7003 -c 127.0.0.1:6002   &> local_node_4.log &
pid4="$!"
echo "Node started as process $pid1, $pid2, $pid3, $pid4"
echo "Wait 5s, then start all threads"
sleep 5
curl -s http://127.0.0.1:7000/tx-generator/start?theta=20000 > /dev/null
curl -s http://127.0.0.1:7000/miner/start?lambda=500 > /dev/null
curl -s http://127.0.0.1:7001/tx-generator/start?theta=20000 > /dev/null
curl -s http://127.0.0.1:7001/miner/start?lambda=500 > /dev/null
curl -s http://127.0.0.1:7002/tx-generator/start?theta=20000 > /dev/null
curl -s http://127.0.0.1:7002/miner/start?lambda=500 > /dev/null
curl -s http://127.0.0.1:7003/tx-generator/start?theta=20000 > /dev/null
curl -s http://127.0.0.1:7003/miner/start?lambda=500 > /dev/null
read -n1 -s -r -p $'Press to kill...\n' key
echo "Auto kill"
kill $pid1
kill $pid2
kill $pid3
kill $pid4