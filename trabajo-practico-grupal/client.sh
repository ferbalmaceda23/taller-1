#!/bin/bash
cargo build
if [[ $1 == '-g' ]]; then
    cargo run -p client
else
    ( cat )  | cargo run -p client -- 127.0.0.1 8080 &
    client_pid=$!
    if [[ $1 == '1' ]]; then
        cat bash/client_1_register.txt > "/proc/$client_pid/fd/0"
    elif [[ $1 == '2' ]]; then
        cat bash/client_2_register.txt > "/proc/$client_pid/fd/0"
    elif [[ $1 == '3' ]]; then
        cat bash/client_3_register.txt > "/proc/$client_pid/fd/0"
    fi
    wait $client_pid
    exit
fi