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
	# Copy epilog.sh to container
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp ./containers/docker-centos7-slurm/epilog.sh slurm:/epilog.sh
	# Copy Slurm epilog collector to container
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp \
		./target/x86_64-unknown-linux-musl/debug/auditor-slurm-epilog-collector \
		slurm:/auditor-slurm-epilog-collector

	docker exec auditor-slurm-1 chown slurm:slurm /auditor-slurm-epilog-collector
	docker exec auditor-slurm-1 chown slurm:slurm /epilog.sh
	docker exec auditor-slurm-1 mkdir /epilog_logs
	docker exec auditor-slurm-1 chown slurm:slurm /epilog_logs

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

function compile_collector() {
	if [ "$RELEASE_MODE" = true ]; then
		RUSTFLAGS='-C link-args=-s' \
			cargo build \
			--target $TARGET_ARCH \
			--bin auditor-slurm-epilog-collector \
			--release
	else
		RUSTFLAGS='-C link-args=-s' \
			cargo build \
			--target $TARGET_ARCH \
			--bin auditor-slurm-epilog-collector
	fi
}

function start_auditor() {
	AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor &
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

SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh

if [[ -z "${SKIP_COMPILATION}" ]]
then
	compile_collector
fi
start_container
start_auditor

docker exec auditor-slurm-1 sbatch --wrap="sleep 1"
sleep 5

# docker exec auditor-slurm-1 scontrol show job 1
# docker exec auditor-slurm-1 ls -la /epilog_logs
docker exec auditor-slurm-1 cat /epilog_logs/epilog.log

curl -vvv http://localhost:8000/get

sleep 2
stop_container
stop_auditor

