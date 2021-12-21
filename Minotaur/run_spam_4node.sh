#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
bash macOS_time.sh
binary_path="./target/release/Minotaur"
time=`cat time.txt`
$binary_path -vv --txnn 0 --txnd 3 --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 &> local_node_1.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$time" &
pid1="$!"
$binary_path -vv --txnn 1 --txnd 3 --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 &> local_node_2.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f3541 --ts "$time" &
pid2="$!"
$binary_path -vv --txnn 2 --txnd 3 --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001 &> local_node_3.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120fa3d4 --ts "$time" &
pid3="$!"
$binary_path -vv --txnn 0 --txnd 3 --p2p 127.0.0.1:6003 --api 127.0.0.1:7003 -c 127.0.0.1:6002 &> local_node_4.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120fb342 --ts "$time"  &
pid4="$!"
echo "Node started as process $pid1, $pid2, $pid3, $pid4"
echo "Wait 5s, then start all threads"
sleep 5
curl -s http://127.0.0.1:7000/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7000/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7000/staker/start?zeta=2000 > /dev/null
curl -s http://127.0.0.1:7001/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7001/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7001/staker/start?zeta=3000 > /dev/null
curl -s http://127.0.0.1:7002/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7002/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7002/staker/start?zeta=3000 > /dev/null
curl -s http://127.0.0.1:7003/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7003/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7003/staker/start?zeta=3000 > /dev/null
read -n1 -s -r -p $'Press to kill...\n' key
echo "Auto kill"
kill $pid1
kill $pid2
kill $pid3
kill $pid4