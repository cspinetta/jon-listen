#!/usr/bin/env bash

cd "${0%/*}"
cd ../

function run_test() {

  PERF_COMMAND=${1}
  SLEEP_TIME=${2}
  APP_PID=${3}
  PORT=${4}

  echo "Starting to send datagrams for ${SLEEP_TIME} seconds."

  yes "22222222222222244445555555" | pv | nc -u -w1 127.0.0.1 ${PORT} &

  yes_pid=$!

  eval "${PERF_COMMAND} -p ${APP_PID} sleep ${SLEEP_TIME}"

  kill ${yes_pid}

  echo "Datagram generation has finished successfully."
}

case "$1" in
  jon)

  PERF_COMMAND=${2}
  SLEEP_TIME=${3}
  PORT=${4:-8080}

  RUST_APP="RUST_LOG=info ./target/release/jon-listen &"

  eval "${RUST_APP}"

  app_pid=$!

  run_test "${PERF_COMMAND}" "${SLEEP_TIME}" "${app_pid}" "${PORT}"

  kill ${app_pid}

  ;;

  desp)

  PERF_COMMAND=${2}
  SLEEP_TIME=${3}
  APP_PID=${4}
  PORT=${5}

  run_test "${PERF_COMMAND}" "${SLEEP_TIME}" "${APP_PID}" "${PORT}"

  ;;

  *)

  echo "Usage:"
  echo "$0 jon [perf_command] [sleep_in_seconds] [port(default: 8080)]"
  echo "$0 desp [perf_command] [sleep_in_seconds] [app_pid] [port]"
  echo ""
  echo "Example:"
  echo "------------------"
  echo "$0 jon 'perf stat -d' 10"
  echo "------------------"
  echo "$0 desp 'perf stat -d' 10 1258 9090"
  echo "------------------"
  echo "..."

  ;;
esac

