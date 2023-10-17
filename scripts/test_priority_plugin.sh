#!/usr/bin/env bash
set -x
set -eo pipefail

# Docker
DOCKER_COMPOSE_FILE=${DOCKER_COMPOSE_FILE:="containers/docker-centos7-slurm/docker-compose.yml"}
DOCKER_PROJECT_DIR=${DOCKER_PROJECT_DIR:="."}
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:="auditor"}
# Plugin build
RELEASE_MODE=${RELEASE_MODE:=false}
TARGET_ARCH=${TARGET_ARCH:="x86_64-unknown-linux-musl"}
DB_NAME=${DB_NAME:=$(uuidgen)}

function stop_container() {
  echo >&2 "Stopping container"
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    down
}

function start_container() {
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    up -d
  # Copy slurm.conf to container
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    cp ./containers/docker-centos7-slurm/slurm.conf slurm:/etc/slurm/slurm.conf

  if [ "$RELEASE_MODE" = true ]; then
    # Copy from one folder if RELEASE_MODE is true
    source_folder="./target/x86_64-unknown-linux-musl/release"
  else
    # Copy from another folder if RELEASE_MODE is not true (false or unset)
    source_folder="./target/x86_64-unknown-linux-musl/debug"
  fi
  # Copy priority plugin to container
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    cp \
    "$source_folder/auditor-priority-plugin" \
    slurm:/auditor-priority-plugin
  # Copy configs for plugin
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    cp ./containers/docker-centos7-slurm/plugin_config_fullspread.yaml slurm:/plugin_config_fullspread.yaml
  docker compose \
    --file $DOCKER_COMPOSE_FILE \
    --project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
    cp ./containers/docker-centos7-slurm/plugin_config_scaledbysum.yaml slurm:/plugin_config_scaledbysum.yaml

  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" chown slurm:slurm /auditor-priority-plugin
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" mkdir /priority_plugin_logs
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" chown slurm:slurm /priority_plugin_logs

  COUNTER=0
  until docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" scontrol ping; do
    echo >&2 "Slurm container is still unavailable - sleeping"
    let COUNTER=COUNTER+1
    if [ "$COUNTER" -gt "30" ]; then
      echo >&2 "Docker container did not come up in time."
      echo >&2 "Docker logs:"
      docker logs "${COMPOSE_PROJECT_NAME}-slurm-1"
      docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" cat /var/log/slurm/slurmctld.log
      stop_container
      echo >&2 "Exiting."
      exit 1
    fi
    sleep 1
  done
}

#!/bin/bash

function compile_priority_plugin() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build --bin auditor-priority-plugin --target x86_64-unknown-linux-musl --release
  else
    cargo build --bin auditor-priority-plugin --target x86_64-unknown-linux-musl
  fi
}

function start_priority_plugin() {
  if [[ -z "${SKIP_COMPILATION}" ]]; then
    compile_priority_plugin
  fi

  local config_file="$1"
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /auditor-priority-plugin /$config_file &

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
  local pid=$(docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" pgrep -f "/auditor-priority-plugin")
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" kill -2 "$pid"
  sleep 2
}

function compile_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build --bin auditor --release
  else
    cargo build --bin auditor
  fi
}

function compile_plugin() {
  if [ "$RELEASE_MODE" = true ]; then
    RUSTFLAGS='-C link-args=-s' \
      cargo build \
      --target $TARGET_ARCH \
      --bin auditor-priority-plugin \
      --release
  else
    RUSTFLAGS='-C link-args=-s' \
      cargo build \
      --target $TARGET_ARCH \
      --bin auditor-priority-plugin
  fi
}

function stop_auditor() {
  echo >&2 "Stopping Auditor"
  kill $AUDITOR_SERVER_PID
}

function start_auditor() {
  if [[ -z "${SKIP_COMPILATION}" ]]; then
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
    echo >&2 "Auditor is still unavailable - sleeping"
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

function fill_auditor() {
  # Group1 (40 * 1.2 * 60 + 40 * 1.5 * 4*60 = 17280)
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "1", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z" }' \
    http://localhost:8000/add
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "2", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group1"]}, "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "value": 1.5 }] }], "start_time": "2022-06-27T16:00:00Z", "stop_time": "2022-06-27T16:04:00Z" }' \
    http://localhost:8000/add

  # Group2 (20 * 1.8 * 8*60 + 10 * 0.8 * 60 = 17760)
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "3", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 20, "scores": [{ "name": "HEPSPEC", "value": 1.8 }] }], "start_time": "2022-06-27T14:00:00Z", "stop_time": "2022-06-27T14:08:00Z" }' \
    http://localhost:8000/add
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "4", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group2"]}, "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "value": 0.8 }] }], "start_time": "2022-06-27T13:00:00Z", "stop_time": "2022-06-27T13:01:00Z" }' \
    http://localhost:8000/add

  # Group3 (80 * 1.0 * 5 * 60 = 24000)
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "5", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group3"]}, "components": [{ "name": "NumCPUs", "amount": 80, "scores": [{ "name": "HEPSPEC", "value": 1.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T12:05:00Z" }' \
    http://localhost:8000/add

  # Group4 (10 * 1.0 * 60 = 600)
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "6", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group4"]}, "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "value": 1.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T12:01:00Z" }' \
    http://localhost:8000/add

  # Group5 (Is not configured and therefore is not allowed to affect the calculation)
  curl --header "Content-Type: application/json" \
    --data '{ "record_id": "7", "meta": {"site_id": ["test"], "user_id": ["stefan"], "group_id": ["group5"]}, "components": [{ "name": "NumCPUs", "amount": 10000, "scores": [{ "name": "HEPSPEC", "value": 100.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T13:01:00Z" }' \
    http://localhost:8000/add
}

function test_priority_plugin_fullspread() {
  sleep 1
  start_priority_plugin plugin_config_fullspread.yaml
  sleep 2
  stop_priority_plugin
  sleep 2

  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part1 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "47041" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part2 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "48348" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part3 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "65335" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part4 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "1634" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part6 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "1" ]; then exit 1; fi
  }

  sleep 2
}

function test_priority_plugin_scaledbysum() {
  sleep 1
  start_priority_plugin plugin_config_scaledbysum.yaml
  sleep 2
  stop_priority_plugin
  sleep 2

  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part1 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "18931" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part2 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "19457" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part3 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "26292" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part4 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "658" ]; then exit 1; fi
  }
  docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /usr/bin/scontrol show Partition=part6 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | {
    read prio
    if [ "$prio" != "1" ]; then exit 1; fi
  }

  sleep 2
}

SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh

cleanup_exit() {
  setsid nohup bash -c "
		docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR down
    kill $AUDITOR_SERVER_PID
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

if [[ -z "${SKIP_COMPILATION}" ]]; then
  compile_plugin
fi
start_container
start_auditor

fill_auditor
test_priority_plugin_fullspread
test_priority_plugin_scaledbysum

stop_container
stop_auditor
