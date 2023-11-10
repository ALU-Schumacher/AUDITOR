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

function setup_python_env() {
	python3 -m venv "$ENV_DIR"
	source "$ENV_DIR/bin/activate"
	pip install --upgrade pip
  pip install requests==2.31.0
}

function start_auditor() {
	if [[ -z "${SKIP_COMPILATION}" ]]
	then
		compile_auditor
	fi
	if [ "$RELEASE_MODE" = true ]; then
		AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME AUDITOR_METRICS__DATABASE__FREQUENCY=5 ./target/release/auditor &
	else
		AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME AUDITOR_METRICS__DATABASE__FREQUENCY=5 ./target/debug/auditor &
	fi
	AUDITOR_SERVER_PID=$!
	COUNTER=0
	until curl http://localhost:8000/health_check; do
		>&2 echo "Auditor is still unavailable - sleeping"
		(( COUNTER=COUNTER+1 ))
		if [ "$COUNTER" -gt "30" ]; then
			echo >&2 "Auditor did not come up in time."
			stop_auditor $AUDITOR_SERVER_PID
			echo >&2 "Exiting."
			exit 1
		fi
		sleep 1
	done
}

function test_auditor_prometheus_exporter() {
  for script in ./auditor/scripts/test_prometheus_exporter/*.py
  do
    DB_NAME=$(uuidgen)
    SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh
    start_auditor
    python3 "$script"
    kill $AUDITOR_SERVER_PID
  done
}

cleanup_exit() {
  setsid nohup bash -c "
    kill $AUDITOR_SERVER_PID
    rm -rf $ENV_DIR
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

compile_auditor
setup_python_env
test_auditor_prometheus_exporter
