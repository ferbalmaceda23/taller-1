#!/bin/bash
if [[ $2 == '-r' ]]; then
    > server/rsc/clients.txt
    > server/rsc/channels.txt
fi
if [[ $1 == '-f' ]]; then
    cargo run -p server -- 8080 main_server
elif [[ $1 == '-c' ]]; then
    cargo run -p server -- 8081 child main_server 127.0.0.1 8080
fi
