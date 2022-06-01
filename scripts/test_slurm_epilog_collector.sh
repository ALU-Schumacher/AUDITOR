#!/usr/bin/env bash
set -x
set -eo pipefail

function start_container() {
	docker compose --file containers/docker-centos7-slurm/docker-compose.yml --project-directory=. up -d

	COUNTER=0
	until docker exec auditor-slurm-1 scontrol ping; do
		>&2 echo "Slurm container is still unavailable - sleeping"
		let COUNTER=COUNTER+1
		if [ "$COUNTER" -gt "30" ]; then
			echo >&2 "Docker container did not come up in time."
			echo >&2 "Docker logs:"
			docker logs auditor-slurm-1
			stop_container
			echo >&2 "Exiting."
			exit 1
		fi
		sleep 1
	done
}

function stop_container() {
	echo >&2 "Stopping container"
	docker compose --file containers/docker-centos7-slurm/docker-compose.yml --project-directory=. down
}

function start_auditor() {
	./target/debug/auditor &
	SERVER_PID=$!
	COUNTER=0
	until curl http://localhost:8000/health_check; do
		>&2 echo "auditor is still unavailable - sleeping"
		let COUNTER=COUNTER+1
		if [ "$COUNTER" -gt "30" ]; then
			echo >&2 "Auditor did not come up in time."
			stop_auditor $SERVER_PID
			echo >&2 "Exiting."
			exit 1
		fi
		sleep 1
	done
}

function stop_auditor() {
	echo >&2 "Stopping Auditor"
	kill $1
}


start_container
start_auditor

# docker ps -a

docker exec auditor-slurm-1 sinfo

stop_container
stop_auditor $SERVER_PID

