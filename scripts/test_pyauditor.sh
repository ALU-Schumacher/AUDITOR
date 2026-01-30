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

function compile_pyauditor() {
  python -m venv $ENV_DIR
  source $ENV_DIR/bin/activate
  pip install maturin
  pip install tzlocal
  pip install patchelf
  if [ "$RELEASE_MODE" = true ]; then
    maturin develop --manifest-path pyauditor/Cargo.toml --release
  else
    maturin develop --manifest-path pyauditor/Cargo.toml
  fi
}

function start_auditor() {
  if [[ -z "${SKIP_COMPILATION}" ]]
  then
    compile_auditor
  fi
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/release/auditor &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor &
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

function test_pyauditor() {
  for script in ./pyauditor/scripts/test_*.py
  do
    DB_NAME=$(uuidgen)
    SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh
    start_auditor
    python3 $script
    echo >&2 "Stopping AUDITOR server"
    if kill -0 "$AUDITOR_SERVER_PID" 2>/dev/null; then
        kill -2 "$AUDITOR_SERVER_PID"
        wait "$AUDITOR_SERVER_PID"
    else
        echo >&2 "Process $$AUDITOR_SERVER_PID does not exist. Nothing to stop."
    fi
  done
}

cleanup_exit() {
  setsid nohup bash -c "
  kill $AUDITOR_SERVER_PID
  if [[ -z \"${SKIP_PYAUDITOR_COMPILATION}\" ]]; then rm -rf $ENV_DIR; fi
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

if [[ -z "${SKIP_PYAUDITOR_COMPILATION}" ]]
then
  compile_pyauditor
fi
test_pyauditor
