#!/usr/bin/env bash
set -x
set -eo pipefail

# Docker
DOCKER_COMPOSE_FILE=${DOCKER_COMPOSE_FILE:="containers/docker-centos7-slurm/docker-compose.yml"}
DOCKER_PROJECT_DIR=${DOCKER_PROJECT_DIR:="."}
COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:="auditor"}
# Collector build
RELEASE_MODE=${RELEASE_MODE:=false}
TARGET_ARCH=${TARGET_ARCH:="x86_64-unknown-linux-musl"}
DB_NAME=${DB_NAME:=$(uuidgen)}
COMMENT="{ 'voms': '/atlas/Role=production', 'subject': '/some/thing' }"


function stop_container () {
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
	# Copy Slurm collector to container
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
		cp \
		./target/${TARGET_ARCH}/debug/auditor-slurm-collector \
		slurm:/auditor-slurm-collector
	# Copy config for collector
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
		cp ./containers/docker-centos7-slurm/collector_config.yaml slurm:/collector_config.yaml
  # Copy tls_config for collector
	docker compose \
		--file $DOCKER_COMPOSE_FILE \
		--project-directory=$DOCKER_PROJECT_DIR \
    --project-name="$COMPOSE_PROJECT_NAME" \
		cp ./containers/docker-centos7-slurm/collector_tls_config.yaml slurm:/collector_tls_config.yaml
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
    --project-name="$COMPOSE_PROJECT_NAME" \
		cp ./containers/docker-centos7-slurm/batch5.sh slurm:/batch.sh

	# docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" chown slurm:slurm /auditor-slurm-collector /collector_config.yaml
	docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" mkdir -p /collector_logs
	docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" chown slurm:slurm /collector_logs

	COUNTER=0
	until docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" scontrol ping; do
		>&2 echo "Slurm container is still unavailable - sleeping"
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
			--bin auditor-slurm-collector \
			--release
	else
		RUSTFLAGS='-C link-args=-s' \
			cargo build \
			--target $TARGET_ARCH \
			--bin auditor-slurm-collector
	fi
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

function start_slurm_collector() {
	if [[ -z "${SKIP_COMPILATION}" ]]
	then
		compile_collector
	fi
	docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" /auditor-slurm-collector /collector_tls_config.yaml &
}

function stop_auditor() {
	echo >&2 "Stopping Auditor"
	kill $AUDITOR_SERVER_PID
}

function test_collector() {
	# Run on partition1
	docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" sh -c "sbatch --job-name=test_part1 --partition=part1 --comment=\"$COMMENT\" /batch.sh" 
	sleep 20

	TEST1=$(curl -X GET https://localhost:8443/records --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem --cacert scripts/certs/rootCA.pem | jq)

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

	if [ "$(echo $TEST1 | jq '.[] | select(.record_id=="slurm-1") | .meta | .voms | .[0]')" != '"%2Fatlas%2FRole=production"' ]
	then
		echo >&2 "Incorrect meta of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_container
		stop_auditor
		exit 1
	fi

	if [ "$(echo $TEST1 | jq '.[] | select(.record_id=="slurm-1") | .meta | .subject | .[0]')" != '"%2Fsome%2Fthing"' ]
	then
		echo >&2 "Incorrect meta of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_container
		stop_auditor
		exit 1
	fi

	if [ $(echo $TEST1 | jq '.[] | select(.record_id=="slurm-1") | .meta | .site_id | .[0]') != '"SiteA"' ]
	then
		echo >&2 "Incorrect site_id of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_container
		stop_auditor
		exit 1
	fi

	# Run on partition2
	docker exec "${COMPOSE_PROJECT_NAME}-slurm-1" sh -c "sbatch --job-name=test_part2 --partition=part2 /batch.sh"
	sleep 20

	TEST2=$(curl -X GET https://localhost:8443/records --cert scripts/certs/client-cert.pem --key scripts/certs/client-key.pem --cacert scripts/certs/rootCA.pem | jq)

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

	if [ $(echo $TEST2 | jq '.[] | select(.record_id=="slurm-2") | .meta | .site_id | .[0]') != '"SiteB"' ]
	then
		echo >&2 "Incorrect site_id of record in accounting database. Returned record:"
		echo >&2 $TEST1
		stop_container
		stop_auditor
		exit 1
	fi
}

SKIP_DOCKER=true POSTGRES_DB=$DB_NAME ./scripts/init_db.sh

cleanup_exit() {
  setsid nohup bash -c "
		docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR --project-name=$COMPOSE_PROJECT_NAME down
    kill $AUDITOR_SERVER_PID
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

if [[ -z "${SKIP_COMPILATION}" ]]
then
	compile_collector
fi
start_container
start_auditor
start_slurm_collector

test_collector

stop_container
stop_auditor

