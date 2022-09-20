#!/usr/bin/sh
# set -x
exec >> /epilog_logs/epilog_${SLURM_JOB_ID}.log
exec 2>> /epilog_logs/epilog_${SLURM_JOB_ID}.log

# curl -v http://localhost:8000/health_check
# curl -vvv http://host.docker.internal:8000/health_check

# Change Auditor host address and port via environment variables
# AUDITOR_ADDR=host.docker.internal AUDITOR_PORT=8000 /auditor-slurm-epilog-collector
# Set DEBUG loglevel (verbose)
# RUST_LOG=debug /auditor-slurm-epilog-collector collector_config.yaml
RUST_LOG=debug AUDITOR_SLURM_EPILOG_COLLECTOR__ADDR=host.docker.internal /auditor-slurm-epilog-collector-client 
echo "EPILOG SCRIPT OF JOB ${SLURM_JOB_ID} DONE"
