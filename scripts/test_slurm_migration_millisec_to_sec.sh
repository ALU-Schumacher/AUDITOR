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

function install_python_deps() {
  cd auditor/scripts/convert_slurm_millisec_to_sec
  python -m venv $ENV_DIR
  source $ENV_DIR/bin/activate
  pip install -r requirements.txt
}

function start_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/release/auditor &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/debug/auditor &
  fi
  AUDITOR_SERVER_PID=$!
  COUNTER=0
  until curl http://localhost:8000/health_check; do
    echo >&2 "Auditor is still unavailable - sleeping"
    ((COUNTER = COUNTER + 1))
    if [ "$COUNTER" -gt "30" ]; then
      echo >&2 "Auditor did not come up in time."
      stop_auditor
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

function cleanup_exit() {
  if [ -n "$AUDITOR_SERVER_PID" ]; then
    echo >&2 "Stopping Auditor due to script exit"
    stop_auditor
  fi
}

function fill_auditor_db() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{"name": "TotalCPU", "amount": 273, "scores": []}], "start_time": "2025-05-01T15:00:00Z", "stop_time": "2025-05-01T15:01:00Z" }' \
    http://localhost:8000/record

  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{"name": "TotalCPU", "amount": 273, "scores": []}], "start_time": "2025-05-01T15:00:00Z", "stop_time": "2025-05-01T15:01:00Z" }' \
    http://localhost:8000/record
}

function convert_slurm_millisec_to_sec() {
  python convert_totalcpu_millisec_to_sec.py
}

function check_if_records_are_correctly_migrated_from_millisec_to_sec() {
  python test_convert_slurm_millisec_to_sec.py
}

trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

start_auditor 

install_python_deps

fill_auditor_db

convert_slurm_millisec_to_sec

check_if_records_are_correctly_migrated_from_millisec_to_sec

stop_auditor
