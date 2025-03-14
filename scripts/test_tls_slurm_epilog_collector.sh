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

function stop_container() {
	echo >&2 "Stopping container"
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		down
}


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
	# Copy config for collector
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp ./containers/docker-centos7-slurm/tls_epilog_collector_config.yaml slurm:/collector_config.yaml
  # Copy client tls certs 
  docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
		cp ./scripts/certs slurm:/client_certs
	# Copy basic batch script
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
		cp ./containers/docker-centos7-slurm/batch.sh slurm:/batch.sh

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

function compile_auditor() {
	if [ "$RELEASE_MODE" = true ]; then
		cargo build --bin auditor --release
	else
		cargo build --bin auditor
	fi
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

function stop_auditor() {
	echo >&2 "Stopping Auditor"
	kill $AUDITOR_SERVER_PID
}

function start_auditor() {
	if [[ -z "${SKIP_COMPILATION}" ]]
	then
		compile_auditor
	fi
	if [ "$RELEASE_MODE" = true ]; then
		AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/release/auditor auditor/configuration/tls_config.yaml &
	else
		AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor auditor/configuration/tls_config.yaml &
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


function test_epilog_collector() {
	# Run on partition1
	docker exec auditor-slurm-1 sbatch --job-name="test_part1" --partition=part1 /batch.sh 
	sleep 5

	docker exec auditor-slurm-1 cat /epilog_logs/epilog.log

	TEST1=$(curl -X GET https://localhost:8443/records --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem --cacert scripts/certs/rootCA.pem | jq -s)

	if [ "$(echo $TEST1 | jq '. | length')" != 1 ]
	then
		echo >&2 "Incorrect number of records in accounting database."
		stop_container
		stop_auditor
		exit 1
	fi

	if [ "$(echo $TEST1 | jq '.[] | select(.record_id=="slurm-1") | .components | .[] | .scores | .[] | .value')" != 1.1 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_container
		stop_auditor
		exit 1
	fi

	# Run on partition2
	docker exec auditor-slurm-1 sbatch --job-name="test_part2" --partition=part2 /batch.sh 
	sleep 5

	TEST2=$(curl -X GET https://localhost:8443/records --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem --cacert scripts/certs/rootCA.pem | jq -s)

	if [ "$(echo $TEST2 | jq '. | length')" != 2 ]
	then
		echo >&2 "Incorrect number of records in accounting database."
		stop_container
		stop_auditor
		exit 1
	fi

	if [ "$(echo $TEST2 | jq '.[] | select(.record_id=="slurm-2") | .components | .[] | .scores | .[] | .value')" != 1.2 ]
	then
		echo >&2 "Incorrect score of record in accounting database. Returned record:"
		echo >&2 $TEST2
		stop_container
		stop_auditor
		exit 1
	fi

	sleep 2
}

SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh

if [[ -z "${SKIP_COMPILATION}" ]]
then
	compile_collector
fi
start_container
start_auditor

test_epilog_collector

stop_container
stop_auditor

