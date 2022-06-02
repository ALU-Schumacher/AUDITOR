#!/usr/bin/env bash
set -x
set -eo pipefail

DOCKER_COMPOSE_FILE="containers/docker-centos7-slurm/docker-compose.yml"
DOCKER_PROJECT_DIR="."

function start_container() {
	docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR up -d
	# Copy slurm.conf to container
	docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR cp ./containers/docker-centos7-slurm/slurm.conf slurm:/etc/slurm/slurm.conf
	# Copy epilog.sh to container
	docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR cp ./containers/docker-centos7-slurm/epilog.sh slurm:/epilog.sh

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
	docker compose --file $DOCKER_COMPOSE_FILE --project-directory=$DOCKER_PROJECT_DIR down
}

function start_auditor() {
	./target/debug/auditor &
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


start_container
start_auditor

# docker ps -a

docker exec auditor-slurm-1 sinfo

stop_container
stop_auditor

