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
  echo >&2 "Stopping AUDITOR server"
  if kill -0 "$AUDITOR_SERVER_PID" 2>/dev/null; then
      kill -2 "$AUDITOR_SERVER_PID"
      wait "$AUDITOR_SERVER_PID"
  else
      echo >&2 "Process $$AUDITOR_SERVER_PID does not exist. Nothing to stop."
  fi
}

cleanup_exit() {
  setsid nohup bash -c "
  if kill -0 ${AUDITOR_SERVER_PID} 2>/dev/null; then
    kill -2 ${AUDITOR_SERVER_PID}
    wait ${AUDITOR_SERVER_PID}
  fi
  "
}

function fill_auditor_db() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{"name": "TotalCPU", "amount": 2733, "scores": []}], "start_time": "2025-05-01T15:00:00Z", "stop_time": "2025-05-01T15:00:02.733Z" }' \
    http://localhost:8000/record

  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{"name": "TotalCPU", "amount": 4577, "scores": []}], "start_time": "2025-05-01T16:00:00Z", "stop_time": "2025-05-01T16:00:04.577Z" }' \
    http://localhost:8000/record

    curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "3", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{"name": "TotalCPU", "amount": 3, "scores": []}], "start_time": "2025-05-01T18:00:00Z", "stop_time": "2025-05-01T18:00:00.003Z" }' \
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
