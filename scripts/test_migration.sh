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

function start_auditor_main() {
  if [[ -z "${SKIP_COMPILATION}" ]]; then
    compile_auditor
  fi
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


function fill_auditor() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z" }' \
    http://localhost:8000/record
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.5 }] }], "start_time": "2022-06-27T16:00:00Z", "stop_time": "2022-06-27T16:04:00Z" }' \
    http://localhost:8000/record
}

function fill_auditor_new() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "3", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 20, "scores": [{ "name": "HEPSPEC", "value": 1.8 }] }], "start_time": "2022-06-27T14:00:00Z", "stop_time": "2023-06-27T14:08:00Z" }' \
    http://localhost:8000/record
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "4", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "value": 0.8 }] }], "start_time": "2022-06-27T13:00:00Z", "stop_time": "2022-06-27T13:01:00Z" }' \
    http://localhost:8000/record

}

function test_collector() {
	TEST1=$(curl -X GET http://localhost:8000/records | jq)

	if [ "$(echo $TEST1 | jq '. | length')" != 4 ]
	then
		echo >&2 "Incorrect number of records in auditor database."
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST1 | jq '.[] | select(.record_id=="1") | .components | .[] | .scores | .[] | .value')" != 1.2 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_auditor
		exit 1
	fi

  expected_json='{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z","runtime":60 }'

  TEST2=$(curl -X GET http://localhost:8000/record/"1" | jq)

  if [ "$(echo "$TEST2" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" != "$(echo "$expected_json" | jq -c 'walk(if type == "object" then to_entries | sort_by(.key) | from_entries else . end)' | tr -d '[:space:]')" ]; then
    echo >&2 "The content of TEST2 does not match the expected JSON data."
    stop_auditor
    exit 1
  fi

}

function auditor_main() {
  SKIP_DOCKER=true ./scripts/init_db.sh 
  ./scripts/init_slurm_collector_sqlite.sh
  ./scripts/init_client_sqlite.sh
  compile_auditor
  start_auditor_main
  fill_auditor
  kill $AUDITOR_SERVER_PID
  wait $AUDITOR_SERVER_PID
}

function auditor_new_migration() {
  sqlx migrate run --source migrations --database-url=postgres://postgres:password@localhost:5432/auditor
  start_auditor_main
  fill_auditor_new
  test_collector
}

function cleanup_exit() {
  if [ -n "$AUDITOR_SERVER_PID" ]; then
    echo >&2 "Stopping Auditor due to script exit"
    kill $AUDITOR_SERVER_PID
    wait $AUDITOR_SERVER_PID
  fi
}

trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

auditor_main  

auditor_new_migration


