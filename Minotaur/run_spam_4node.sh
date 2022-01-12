#!/bin/bash
#build --release (with debug symbols)
#increase staker's block size if needed
binary_path="./target/release/Minotaur"
for delay in 0 100 200 300
do
    bash macOS_time.sh
    time=`cat time.txt`
    $binary_path -vv --txnn 0 --txnd 3 --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 &> local_node_1_delay$delay.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$time" --delay $delay &
    pid1="$!"
    $binary_path -vv --txnn 1 --txnd 3 --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 &> local_node_2_delay$delay.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f3541 --ts "$time" --delay $delay &
    pid2="$!"
    $binary_path -vv --txnn 2 --txnd 3 --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001 &> local_node_3_delay$delay.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120fa3d4 --ts "$time" --delay $delay &
    pid3="$!"
    $binary_path -vv --txnn 0 --txnd 3 --p2p 127.0.0.1:6003 --api 127.0.0.1:7003 -c 127.0.0.1:6002 &> local_node_4_delay$delay.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120fb342 --ts "$time" --delay $delay &
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
    # read -n1 -s -r -p $'Press to kill...\n' key
    echo "sleep 3000s"
    sleep 3000
    curl -s http://127.0.0.1:7000/ledger/spam >> spam_result_delay$delay.txt
    echo '' >> spam_result_delay$delay.txt 
    curl -s http://127.0.0.1:7001/ledger/spam >> spam_result_delay$delay.txt
    echo '' >> spam_result_delay$delay.txt 
    curl -s http://127.0.0.1:7002/ledger/spam >> spam_result_delay$delay.txt
    echo '' >> spam_result_delay$delay.txt 
    curl -s http://127.0.0.1:7003/ledger/spam >> spam_result_delay$delay.txt
    echo '' >> spam_result_delay$delay.txt 
    echo "Auto kill"
    kill $pid1
    kill $pid2
    kill $pid3
    kill $pid4
    echo "Sleep 30s then start new experiment"
    sleep 30
done