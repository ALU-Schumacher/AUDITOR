#!/usr/bin/env bash
#!/usr/bin/env bash
set -x
set -eo pipefail

RELEASE_MODE=${RELEASE_MODE:=false}
ENV_DIR=${ENV_DIR:=".env_test"}

function compile_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build --bin auditor --release
  else
    cargo build --bin auditor
  fi
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
    --data '{"record_id": "record1", "meta": {"site_id": ["site%2F1"], "user_id": ["user%2F1"], "group_id": ["group%2F1"]}, "components": [{"name": "NumCPUs", "amount": 31, "scores": [{"name": "HEPSPEC", "value": 1.2}]}], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z", "runtime": 60}' \
    http://localhost:8000/record

  curl -X POST --header "Content-Type: application/json" \
    --data '{"record_id": "record2", "meta": {"site_id": ["site%2F2"], "user_id": ["user%2F2"], "group_id": ["group%2F2"]}, "components": [{"name": "NumCPUs", "amount": 31, "scores": [{"name": "HEPSPEC", "value": 1.2}]}], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z", "runtime": 60}' \
    http://localhost:8000/record
}

function replace_encoded_string_in_db() {
  cd auditor/scripts/slurm_revert_encoding/
  cargo run
}

function check_if_records_are_correctly_reverted() {
  python ../test_valid_names/test_slurm_decoding.py
}

trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

start_auditor 

fill_auditor_db

replace_encoded_string_in_db

check_if_records_are_correctly_reverted

stop_auditor
