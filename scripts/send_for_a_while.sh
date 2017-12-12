#!/usr/bin/env bash

PORT=${1}
SLEEP_TIME=$2

echo "Starting to send datagrams for ${SLEEP_TIME} seconds."

yes "22222222222222244445555555" | pv | nc -u -w1 127.0.0.1 ${PORT} &

pid=$!

sleep ${SLEEP_TIME} # in seconds

kill ${pid}

echo "Datagram generation has finished successfully."
