var=`cat time.txt`
RUST_BACKTRACE=full cargo run --release -- -vv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 --sk c9afa9d845ba75166b5c215767b1d6934e50c3db36e89b127b8a622b120f6721 --ts "$var" 
