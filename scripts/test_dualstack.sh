#!/usr/bin/env bash
set -x
set -eo pipefail

RELEASE_MODE=${RELEASE_MODE:=false}
ENV_DIR=${ENV_DIR:=".env_test"}

function compile_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build -p auditor --release
  else
    cargo build -p auditor
  fi
}

function start_auditor_with_env() {
  if [[ -z "${SKIP_COMPILATION}" ]]
  then
    compile_auditor
  fi
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0,::1 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/release/auditor auditor/configuration/base.yaml &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0,::1 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor auditor/configuration/base.yaml &
  fi
  AUDITOR_SERVER_PID=$!
  COUNTER=0
  until curl http://localhost:8000/health_check; do
    >&2 echo "Auditor is still unavailable - sleeping"
    let COUNTER=COUNTER+1
    if [ "$COUNTER" -gt "30" ]; then
      echo >&2 "Auditor did not come up in time."
      stop_auditor $AUDITOR_SERVER_PID
      echo >&2 "Exiting."
      exit 1
    fi
    sleep 1
  done
}

function start_auditor_with_config() {
  if [[ -z "${SKIP_COMPILATION}" ]]
  then
    compile_auditor
  fi
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/release/auditor auditor/configuration/dualstack.yaml &
  else
    AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor auditor/configuration/dualstack.yaml &
  fi
  AUDITOR_SERVER_PID=$!
  COUNTER=0
  until curl http://localhost:8000/health_check; do
    >&2 echo "Auditor is still unavailable - sleeping"
    let COUNTER=COUNTER+1
    if [ "$COUNTER" -gt "30" ]; then
      echo >&2 "Auditor did not come up in time."
      stop_auditor $AUDITOR_SERVER_PID
      echo >&2 "Exiting."
      exit 1
    fi
    sleep 1
  done
}

function stop_auditor() {
  echo >&2 "Stopping Auditor"
  kill $AUDITOR_SERVER_PID
  wait $AUDITOR_SERVER_PID
}

function check_tcp_connection() {
  if [ "$#" -ne 3 ]; then
    echo "Usage: $0 <IPv4_address> <IPv6_address> <port>"
    exit 1
fi

IPv4_ADDRESS=$1
IPv6_ADDRESS=$2

PORT=$3

# Check IPv4 connection
echo "Checking TCP connection to IPv4 address $IPv4_ADDRESS on port $PORT..."
nc -zv -w 3 $IPv4_ADDRESS $PORT
if [ $? -eq 0 ]; then
    echo "IPv4 connection to $IPv4_ADDRESS:$PORT is successful."
else
    echo "IPv4 connection to $IPv4_ADDRESS:$PORT failed."
fi

# Check IPv6 connection
echo "Checking TCP connection to IPv6 address $IPv6_ADDRESS on port $PORT..."
nc -zv -w 3 -6 $IPv6_ADDRESS $PORT
if [ $? -eq 0 ]; then
    echo "IPv6 connection to $IPv6_ADDRESS:$PORT is successful."
else
    echo "IPv6 connection to $IPv6_ADDRESS:$PORT failed."
fi
}

cleanup_exit() {
  setsid nohup bash -c "
  kill $AUDITOR_SERVER_PID
  if [[ -z \"${SKIP_PYAUDITOR_COMPILATION}\" ]]; then rm -rf $ENV_DIR; fi
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

start_auditor_with_env

check_tcp_connection "0.0.0.0" "::1" "8000"

stop_auditor

sleep 2

start_auditor_with_config

check_tcp_connection "0.0.0.0" "::1" "8000"

