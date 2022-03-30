#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
binary_path="./target/release/Minotaur"
bash macOS_time.sh
time=`cat time.txt`
$binary_path -vv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 > /dev/null 2>&1 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$time" --delay $delay &
pid1="$!"
$binary_path -vv --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 > /dev/null 2>&1 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f3541 --ts "$time" --delay $delay &
pid2="$!"
$binary_path -vv --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001 > /dev/null 2>&1 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120fa3d4 --ts "$time" --delay $delay &
pid3="$!"
echo "Node started as process $pid1, $pid2, $pid3"
echo "Wait 10s, then start all threads"
sleep 10
curl -s http://127.0.0.1:7000/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7000/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7000/staker/start?zeta=2000 > /dev/null
curl -s http://127.0.0.1:7001/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7001/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7001/staker/start?zeta=3000 > /dev/null
curl -s http://127.0.0.1:7002/tx-generator/start?theta=4000 > /dev/null
curl -s http://127.0.0.1:7002/miner/start?lambda=300 > /dev/null
curl -s http://127.0.0.1:7002/staker/start?zeta=3000 > /dev/null
# read -n1 -s -r -p $'Press to kill...\n' key
echo "sleep 30s and let processes run"
sleep 30
curl -s http://127.0.0.1:7000/blockchain/longest-chain >> tmp.txt
echo '' >> tmp.txt 
curl -s http://127.0.0.1:7001/blockchain/longest-chain >> tmp.txt
echo '' >> tmp.txt 
curl -s http://127.0.0.1:7002/blockchain/longest-chain >> tmp.txt
echo '' >> tmp.txt 
echo "Auto kill all processes"
kill $pid1
kill $pid2
kill $pid3