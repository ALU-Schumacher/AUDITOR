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

function start_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/release/auditor auditor/configuration/tls_config.yaml &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/debug/auditor auditor/configuration/tls_config.yaml &
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


function fill_auditor_with_tls_certs() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z" }' \
    https://localhost:8443/record --cacert scripts/certs/rootCA.pem --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.5 }] }], "start_time": "2022-06-27T16:00:00Z", "stop_time": "2022-06-27T16:04:00Z" }' \
    https://localhost:8443/record --cacert scripts/certs/rootCA.pem --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem
}

function test_records() {
 expected_json_1='{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z","runtime":60 }'

  TEST1=$(curl -X GET http://localhost:8000/record/"1" | jq)

  if [ "$(echo "$TEST1" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" != "$(echo "$expected_json_1" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" ]; then
    echo >&2 "The content of TEST1 does not match the expected JSON data."
    stop_auditor
    exit 1
  fi

  expected_json_2='{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.5 }] }], "start_time": "2022-06-27T16:00:00Z", "stop_time": "2022-06-27T16:04:00Z","runtime":240 }'

  TEST2=$(curl -X GET http://localhost:8000/record/"2" | jq)

  if [ "$(echo "$TEST2" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" != "$(echo "$expected_json_2" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" ]; then
    echo >&2 "The content of TEST2 does not match the expected JSON data."
    stop_auditor
    exit 1
  fi

}

function cleanup_exit() {
  if [ -n "$AUDITOR_SERVER_PID" ]; then
    echo >&2 "Stopping Auditor due to script exit"
    stop_auditor
  fi
}

trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

start_auditor

fill_auditor_with_tls_certs

test_records
