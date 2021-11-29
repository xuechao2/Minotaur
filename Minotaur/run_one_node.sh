#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
bash macOS_time.sh
var=`cat time.txt`
binary_path="./target/release/Minotaur"
$binary_path -vv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$var" &> variable_diff_node.log &
pid="$!"
echo "Node started as process $pid"
echo "Wait 2s, then start all threads"
sleep 2
curl -s http://127.0.0.1:7000/tx-generator/start?theta=15000 > /dev/null
curl -s http://127.0.0.1:7000/miner/start?lambda=240 > /dev/null
curl -s http://127.0.0.1:7000/staker/start?zeta=100 > /dev/null

sleep 120
curl -s http://127.0.0.1:7000/miner/start?lambda=500 > /dev/null

sleep 120
curl -s http://127.0.0.1:7000/miner/start?lambda=1000 > /dev/null

sleep 120
curl -s http://127.0.0.1:7000/miner/start?lambda=2000 > /dev/null

sleep 120
curl -s http://127.0.0.1:7000/miner/start?lambda=4000 > /dev/null

sleep 120
read -n1 -s -r -p $'Press to kill...\n' key
echo "Auto kill"
kill $pid