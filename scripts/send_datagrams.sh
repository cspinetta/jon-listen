#!/usr/bin/env bash

count=${1:-10000}

echo "Sending $count datagram's..."

for i in `seq 1 ${count}`;
  do
    echo "22222222222222244445555555" | nc -u -w1 127.0.0.1 8080
  done

echo "Datagram generation has finished successfully."
