#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
bash macOS_time.sh
var=`cat time.txt`
binary_path="./target/release/Minotaur"
$binary_path -vvv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$var" &> local_node_1.log &
pid="$!"
echo "Node $i started as process $pid"
echo "Wait 5s, then start all threads"
sleep 5
curl -s http://127.0.0.1:7000/tx-generator/start?theta=10000 > /dev/null
curl -s http://127.0.0.1:7000/miner/start?lambda=0 > /dev/null
curl -s http://127.0.0.1:7000/staker/start?zeta=100 > /dev/null
echo "Wait 30s, then start dtrace for 60s"
sleep 30
sudo dtrace -x ustackframes=100 -n "profile-997 /pid == $pid/ { @[ustack()] = count(); } tick-60s { exit(0); }"  -o out.user_stacks
# 997 Hz is the frequency
# read -n1 -s -r -p $'Press to continue...\n' key
echo "Auto kill $pid"
kill $pid
cat out.user_stacks | inferno-collapse-dtrace > stacks.folded
cat stacks.folded | rustfilt | c++filt | inferno-flamegraph > flamegraph.svg