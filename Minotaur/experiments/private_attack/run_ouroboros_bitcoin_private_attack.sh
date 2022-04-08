binary_path="./target/release/Minotaur"
log_path="private_attack_experiment_ouro0"
mkdir -p $log_path
for betas in 0 1 2 3 4 5 6 7 8 9 10
do
    if [ "$betas" == "0" ]; then
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
    bash macOS_time.sh
    time=`cat time.txt`
    $binary_path -vv -w 0 --betas $bs1 --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 &> $log_path/$betas-honest.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$time" &
    pid1="$!"
    $binary_path -vv -w 0 --betas $bs2 --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000 &> $log_path/$betas-attacker.log --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f3541 --ts "$time" --atttime 1 &
    pid2="$!"
    echo "Node started as process $pid1, $pid2"
    echo "Wait 2s, then start all threads"
    sleep 2
    curl -s http://127.0.0.1:7000/staker/start?zeta=100 > /dev/null
    curl -s http://127.0.0.1:7001/staker/start?zeta=100 > /dev/null
    # read -n1 -s -r -p $'Press to kill...\n' key
    echo "sleep 120s"
    sleep 120
    echo "Auto kill"
    kill $pid1
    kill $pid2
    echo "Sleep 2s then start new experiment"
    sleep 2
done
