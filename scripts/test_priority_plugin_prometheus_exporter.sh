#!/usr/bin/env bash
set -x
set -eo pipefail

RELEASE_MODE=${RELEASE_MODE:=false}
ENV_DIR=${ENV_DIR:=".env_test"}

function setup_python_env() {
  python3 -m venv "$ENV_DIR"
  source "$ENV_DIR/bin/activate"
  pip install --upgrade pip
  pip install requests==2.31.0
}

function compile_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build -p auditor --release
  else
    cargo build -p auditor
  fi
}

function start_auditor() {
  if [[ -z "${SKIP_COMPILATION}" ]]; then
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

function compile_priority_plugin() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build -p auditor-priority-plugin --release
  else
    cargo build -p auditor-priority-plugin
  fi
}

function start_priority_plugin() {
  if [[ -z "${SKIP_COMPILATION}" ]]; then
    compile_priority_plugin
  fi

  if [ "$RELEASE_MODE" = true ]; then
    ./target/release/auditor-priority-plugin containers/docker-centos7-slurm/test_config_prometheus_exporter.yaml &
  else
    ./target/debug/auditor-priority-plugin containers/docker-centos7-slurm/test_config_prometheus_exporter.yaml &
  fi

  PRIORITY_PLUGIN=$!
  COUNTER=0

  until curl http://localhost:9000/metrics; do
    echo >&2 "Priority plugin is still unavailable - sleeping"
    ((COUNTER = COUNTER + 1))

    if [ "$COUNTER" -gt 30 ]; then
      echo >&2 "Priority plugin did not come up in time"
      stop_priority_plugin
      echo >&2 "Exiting."
      exit 1
    fi

    sleep 1
  done
}

function stop_priority_plugin() {
  echo >&2 "Stopping Priority plugin"
  kill -2 "$PRIORITY_PLUGIN"
  wait "$PRIORITY_PLUGIN"
}

function test_priority_plugin_prometheus_exporter() {
  DB_NAME=$(uuidgen)
  SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh
  start_auditor
  sleep 2

  for script in ./plugins/priority/scripts/test_prometheus_exporter/*.py; do
    start_priority_plugin
    sleep 2
    python3 "$script"
    sleep 3
    stop_priority_plugin
    sleep 2
  done

  stop_auditor
}

cleanup_exit() {
  setsid nohup bash -c "
    kill $PRIORITY_PLUGIN
    kill $AUDITOR_SERVER_PID
    rm -rf $ENV_DIR
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

compile_auditor
compile_priority_plugin
setup_python_env
test_priority_plugin_prometheus_exporter
