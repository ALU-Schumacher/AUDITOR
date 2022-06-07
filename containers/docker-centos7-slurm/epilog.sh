#!/usr/bin/sh
# set -x
exec >> /epilog_logs/epilog.log
exec 2>> /epilog_logs/epilog.log

# curl -v http://localhost:8000/health_check
curl -vvv http://host.docker.internal:8000/health_check

/auditor-slurm-epilog-collector
