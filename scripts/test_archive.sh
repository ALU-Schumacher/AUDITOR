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

function start_auditor_without_archival_service() {
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/release/auditor auditor/configuration/base.yaml &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/debug/auditor auditor/configuration/base.yaml &
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

function start_auditor_with_archival_service() {
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_ARCHIVAL_CONFIG__CRON_SCHEDULE="*/20 * * * * *" AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/release/auditor auditor/configuration/archive.yaml &

  else
    AUDITOR_ARCHIVAL_CONFIG__CRON_SCHEDULE="*/20 * * * * *" AUDITOR_APPLICATION__ADDR=0.0.0.0 ./target/debug/auditor auditor/configuration/archive.yaml &
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

# change the dates

function fill_auditor_db_group1() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2025-05-01T15:00:00Z", "stop_time": "2025-05-01T15:01:00Z" }' \
    http://localhost:8000/record
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.5 }] }], "start_time": "2025-05-31T16:00:00Z", "stop_time": "2025-05-31T16:04:00Z" }' \
    http://localhost:8000/record
}

function fill_auditor_db_group2() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "3", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 20, "scores": [{ "name": "HEPSPEC", "value": 1.8 }] }], "start_time": "2022-06-01T14:00:00Z", "stop_time": "2023-06-01T14:08:00Z" }' \
    http://localhost:8000/record
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "4", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "value": 0.8 }] }], "start_time": "2022-06-30T13:00:00Z", "stop_time": "2022-06-30T13:01:00Z" }' \
    http://localhost:8000/record

}

function fill_auditor_db_group3() {
  start_time1=$(date -u -d "$(date +%Y-%m-01) -1 month" +"%Y-%m-%dT%H:%M:%SZ")
  stop_time1=$(date -u -d "$(date +%Y-%m-01) -1 month +8 minutes" +"%Y-%m-%dT%H:%M:%SZ")
  start_time2=$(date -u -d "$(date +%Y-%m-01) -1 month" +"%Y-%m-%dT%H:%M:%SZ")
  stop_time2=$(date -u -d "$(date +%Y-%m-01) -1 month + 1 minute" +"%Y-%m-%dT%H:%M:%SZ")
  
  curl -X POST --header "Content-Type: application/json" \
    --data "{ \"record_id\": \"5\", \"meta\": {\"site_id\": [\"test\"], \"user_id\": [\"raghuvar\"], \"group_id\": [\"group2\"]}, \"components\": [{ \"name\": \"NumCPUs\", \"amount\": 20, \"scores\": [{ \"name\": \"HEPSPEC\", \"value\": 1.8 }] }], \"start_time\": \"$start_time1\", \"stop_time\": \"$stop_time1\" }" \
    http://localhost:8000/record
    
  curl -X POST --header "Content-Type: application/json" \
    --data "{ \"record_id\": \"6\", \"meta\": {\"site_id\": [\"test\"], \"user_id\": [\"raghuvar\"], \"group_id\": [\"group2\"]}, \"components\": [{ \"name\": \"NumCPUs\", \"amount\": 10, \"scores\": [{ \"name\": \"HEPSPEC\", \"value\": 0.8 }] }], \"start_time\": \"$start_time2\", \"stop_time\": \"$stop_time2\" }" \
    http://localhost:8000/record
}

function fill_auditor_db_group4() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "7", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 20, "scores": [{ "name": "HEPSPEC", "value": 1.8 }] }], "start_time": "2025-01-01T14:00:00Z", "stop_time": "2025-01-01T14:08:00Z" }' \
    http://localhost:8000/record
  curl -X POST --header "Content-Type: application/json" \
    --data '{ "record_id": "8", "meta": {"site_id": ["test"], "user_id": ["raghuvar"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "value": 0.8 }] }], "start_time": "2025-01-30T13:00:00Z", "stop_time": "2025-01-30T13:01:00Z" }' \
    http://localhost:8000/record
}

function auditor_adding_records() {
  start_auditor_without_archival_service
  fill_auditor_db_group1
  fill_auditor_db_group2
  fill_auditor_db_group3
  stop_auditor
  compiling_rust_parquet_restore_script
}

function run_archive() {
  start_auditor_with_archival_service
  check_if_records_are_deleted
}

function run_parquet_to_auditor_db() {
  restore_parquet_to_auditor_db
  check_if_records_are_restored_by_rust_script
  
  fill_auditor_db_group4

  sleep 25

  #restore_parquet_to_auditor_db_to_check_cron_schedule
  compile_python
  run_python_parquet_to_auditor_script
  
  check_if_records_group_4_records_are_restored

  stop_auditor
}

function check_if_records_are_deleted() {
  TEST=$(curl -X GET http://localhost:8000/records | jq -s)

	if [ "$(echo $TEST | jq '. | length')" != 2 ]
	then
		echo >&2 "Incorrect number of records in auditor database."
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST | jq '.[] | select(.record_id=="5") | .components | .[] | .scores | .[] | .value')" != 1.8 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST
		stop_auditor
		exit 1
	fi
}

function check_if_records_are_restored_by_rust_script() {
  TEST=$(curl -X GET http://localhost:8000/records | jq -s)

	if [ "$(echo $TEST | jq '. | length')" != 4 ]
	then
		echo >&2 "Incorrect number of records in auditor database."
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST | jq '.[] | select(.record_id=="2") | .components | .[] | .scores | .[] | .value')" != 1.5 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST | jq '.[] | select(.record_id=="1") | .components | .[] | .scores | .[] | .value')" != 1.2 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST
		stop_auditor
		exit 1
	fi
}

function compiling_rust_parquet_restore_script() {
  cd auditor/scripts/parquet_to_auditor/rust_script
  cargo build
  cd ../../../../
}

function restore_parquet_to_auditor_db() {
  cd auditor/scripts/parquet_to_auditor/rust_script
  cargo run -- ./configuration/config.yaml
}

function restore_parquet_to_auditor_db_to_check_cron_schedule() {
  file_path="../../../../archived_records/auditor_2025_1.parquet" cargo run
}

function check_if_records_group_4_records_are_restored() {
  TEST=$(curl -X GET http://localhost:8000/records | jq -s)

	if [ "$(echo $TEST | jq '. | length')" != 4 ]
	then
		echo >&2 "Incorrect number of records in auditor database."
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST | jq '.[] | select(.record_id=="7") | .components | .[] | .scores | .[] | .value')" != 1.8 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST
		stop_auditor
		exit 1
	fi

  if [ "$(echo $TEST | jq '.[] | select(.record_id=="8") | .components | .[] | .scores | .[] | .value')" != 0.8 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST
		stop_auditor
		exit 1
	fi
}


function compile_python() {
  python -m venv $ENV_DIR
  source $ENV_DIR/bin/activate
  cd ../python_script
  pip install -r requirements.txt
}

function run_python_parquet_to_auditor_script() {
  python3 parquet_to_auditor.py
}

auditor_adding_records

run_archive

run_parquet_to_auditor_db

function stop_auditor() {
  echo >&2 "Stopping Auditor"
  kill $AUDITOR_SERVER_PID
  wait $AUDITOR_SERVER_PID
}

