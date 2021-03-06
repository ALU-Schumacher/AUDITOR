#!/usr/bin/env bash
set -x
set -eo pipefail

# Docker
DOCKER_COMPOSE_FILE=${DOCKER_COMPOSE_FILE:="containers/docker-centos7-slurm/docker-compose.yml"}
DOCKER_PROJECT_DIR=${DOCKER_PROJECT_DIR:="."}
# Collector build
RELEASE_MODE=${RELEASE_MODE:=false}
TARGET_ARCH=${TARGET_ARCH:="x86_64-unknown-linux-musl"}
DB_NAME=${DB_NAME:=$(uuidgen)}

function start_container() {
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		up -d
	# Copy slurm.conf to container
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp ./containers/docker-centos7-slurm/slurm.conf slurm:/etc/slurm/slurm.conf
	# Copy Slurm epilog collector to container
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp \
		./target/x86_64-unknown-linux-musl/debug/auditor-priority-plugin \
		slurm:/auditor-priority-plugin
	# Copy config for collector
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp ./containers/docker-centos7-slurm/plugin_config.yaml slurm:/plugin_config.yaml

	docker exec auditor-slurm-1 chown slurm:slurm /auditor-priority-plugin
	docker exec auditor-slurm-1 mkdir /priority_plugin_logs
	docker exec auditor-slurm-1 chown slurm:slurm /priority_plugin_logs

	COUNTER=0
	until docker exec auditor-slurm-1 scontrol ping; do
		>&2 echo "Slurm container is still unavailable - sleeping"
		let COUNTER=COUNTER+1
		if [ "$COUNTER" -gt "30" ]; then
			echo >&2 "Docker container did not come up in time."
			echo >&2 "Docker logs:"
			docker logs auditor-slurm-1
			docker exec auditor-slurm-1 cat /var/log/slurm/slurmctld.log
			stop_container
			echo >&2 "Exiting."
			exit 1
		fi
		sleep 1
	done
}

function stop_container() {
	echo >&2 "Stopping container"
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		down
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

function stop_auditor() {
	echo >&2 "Stopping Auditor"
	kill $AUDITOR_SERVER_PID
}

function test_priority_plugin() {
	# Group1 (40 * 1.2 * 60 + 40 * 1.5 * 4*60 = 17280)
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "1", "site_id": "test", "user_id": "stefan", "group_id": "group1", "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "factor": 1.2 }] }], "start_time": "2022-06-27T15:00:00Z", "stop_time": "2022-06-27T15:01:00Z" }' \
                http://localhost:8000/add
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "2", "site_id": "test", "user_id": "stefan", "group_id": "group1", "components": [{ "name": "NumCPUs", "amount": 40, "scores": [{ "name": "HEPSPEC", "factor": 1.5 }] }], "start_time": "2022-06-27T16:00:00Z", "stop_time": "2022-06-27T16:04:00Z" }' \
                http://localhost:8000/add

	# Group2 (20 * 1.8 * 8*60 + 10 * 0.8 * 60 = 17760)
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "3", "site_id": "test", "user_id": "stefan", "group_id": "group2", "components": [{ "name": "NumCPUs", "amount": 20, "scores": [{ "name": "HEPSPEC", "factor": 1.8 }] }], "start_time": "2022-06-27T14:00:00Z", "stop_time": "2022-06-27T14:08:00Z" }' \
                http://localhost:8000/add
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "4", "site_id": "test", "user_id": "stefan", "group_id": "group2", "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "factor": 0.8 }] }], "start_time": "2022-06-27T13:00:00Z", "stop_time": "2022-06-27T13:01:00Z" }' \
                http://localhost:8000/add

	# Group3 (80 * 1.0 * 5 * 60 = 24000)
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "5", "site_id": "test", "user_id": "stefan", "group_id": "group3", "components": [{ "name": "NumCPUs", "amount": 80, "scores": [{ "name": "HEPSPEC", "factor": 1.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T12:05:00Z" }' \
                http://localhost:8000/add

	# Group4 (10 * 1.0 * 60 = 600)
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "6", "site_id": "test", "user_id": "stefan", "group_id": "group4", "components": [{ "name": "NumCPUs", "amount": 10, "scores": [{ "name": "HEPSPEC", "factor": 1.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T12:01:00Z" }' \
                http://localhost:8000/add

  # Group5 (Is not configured and therefore is not allowed to affect the calculation)
	curl --header "Content-Type: application/json" \
                --data '{ "record_id": "7", "site_id": "test", "user_id": "stefan", "group_id": "group5", "components": [{ "name": "NumCPUs", "amount": 10000, "scores": [{ "name": "HEPSPEC", "factor": 100.0 }] }], "start_time": "2022-06-27T12:00:00Z", "stop_time": "2022-06-27T13:01:00Z" }' \
                http://localhost:8000/add


	sleep 1
	docker exec -e RUST_LOG=debug auditor-slurm-1 /auditor-priority-plugin plugin_config.yaml
	sleep 2

	docker exec auditor-slurm-1 /usr/bin/scontrol show Partition=part1 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | { read prio; if [ "$prio" != "47041" ]; then exit 1; fi }
	docker exec auditor-slurm-1 /usr/bin/scontrol show Partition=part2 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | { read prio; if [ "$prio" != "48348" ]; then exit 1; fi }
	docker exec auditor-slurm-1 /usr/bin/scontrol show Partition=part3 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | { read prio; if [ "$prio" != "65335" ]; then exit 1; fi }
	docker exec auditor-slurm-1 /usr/bin/scontrol show Partition=part4 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | { read prio; if [ "$prio" !=  "1634" ]; then exit 1; fi }
	docker exec auditor-slurm-1 /usr/bin/scontrol show Partition=part6 | grep PriorityJobFactor | awk '{print $1}' | awk -F "=" '{print $2}' | { read prio; if [ "$prio" !=     "1" ]; then exit 1; fi }

	sleep 2
}

SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh

if [[ -z "${SKIP_COMPILATION}" ]]
then
	compile_plugin
fi
start_container
start_auditor

test_priority_plugin

stop_container
stop_auditor

