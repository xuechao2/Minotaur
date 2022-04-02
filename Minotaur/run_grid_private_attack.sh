binary_path="./target/release/Minotaur"
for betas in 0 1 2 3 4 5 6 7 8 9 10
do
for betaw in 0 1 2 3 4 5 6 7 8 9 10
do
    if [ "$((betaw+betas))" == "0" ]; then
        echo "Skip attacker with 0 0"
        continue
    fi
    if [ "$betas" == "10" ]; then
        bs2="1"
        bs1="0"
    elif [ "$betas" == "0" ]; then
        bs2="0"
        bs1="1"
    else
        bs2="0.$betas"
        bs1="0.$((10-betas))"
    fi
    if [ "$betaw" == "10" ]; then
        bw2="1"
        bw1="0"
    elif [ "$betaw" == "0" ]; then
        bw2="0"
        bw1="1"
    else
        bw2="0.$betaw"
        bw1="0.$((10-betaw))"
    fi
    bash macOS_time.sh
    time=`cat time.txt`
    $binary_path -vv -w 0.5 --betas $bs1 --betaw $bw1 --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 &> private_attack_experiment/$betas-$betaw-honest.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$time" &
    pid1="$!"
    $binary_path -vv -w 0.5 --betas $bs2 --betaw $bw2 --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 &> private_attack_experiment/$betas-$betaw-attacker.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f3541 --ts "$time" --atttime 120000000 &
    pid2="$!"
    echo "Node started as process $pid1, $pid2"
    echo "Wait 2s, then start all threads"
    sleep 2
    curl -s http://127.0.0.1:7000/tx-generator/start?theta=10000 > /dev/null
    curl -s http://127.0.0.1:7000/miner/start?lambda=240 > /dev/null
    curl -s http://127.0.0.1:7000/staker/start?zeta=100 > /dev/null
    curl -s http://127.0.0.1:7001/tx-generator/start?theta=10000 > /dev/null
    curl -s http://127.0.0.1:7001/miner/start?lambda=240 > /dev/null
    curl -s http://127.0.0.1:7001/staker/start?zeta=100 > /dev/null
    # read -n1 -s -r -p $'Press to kill...\n' key
    echo "sleep 240s"
    sleep 240
    echo "Auto kill"
    kill $pid1
    kill $pid2
    echo "Sleep 2s then start new experiment"
    sleep 2
done
done