#!/bin/bash

set -e -o pipefail

run_auditor() {
  ./auditor "$@"
}

run_sqlx_migration() {

  AUDITOR_DATABASE__USERNAME="${AUDITOR_DATABASE__USERNAME:=postgres}"
  AUDITOR_DATABASE__PASSWORD="${AUDITOR_DATABASE__PASSWORD:=password}"
  AUDITOR_DATABASE__HOST="${AUDITOR_DATABASE__HOST:=localhost}"
  AUDITOR_DATABASE__PORT="${AUDITOR_DATABASE__PORT:=5432}"
  AUDITOR_DATABASE__DATABASE_NAME="${AUDITOR_DATABASE__DATABASE_NAME:=auditor}"

  export DATABASE_URL=postgres://${AUDITOR_DATABASE__USERNAME}:${AUDITOR_DATABASE__PASSWORD}@${AUDITOR_DATABASE__HOST}:${AUDITOR_DATABASE__PORT}/${AUDITOR_DATABASE__DATABASE_NAME}
  ./sqlx database create
  ./sqlx migrate run
}

help() {
  echo "Available commands:"
  echo "  auditor: Run AUDITOR"
  echo "  migrate: Run sqlx migrations"
  echo "  shell: Start a shell session (for debugging)"
  echo "  help: Show this help message"
}

if [ $# -eq 0 ]; then
  run_auditor
fi

command="$1"
shift

if [ "$command" = "auditor" ]; then
  run_auditor "$@"
elif [ "$command" = "migrate" ]; then
  run_sqlx_migration
elif [ "$command" = "shell" ]; then
  /bin/bash
elif [ "$command" = "help" ]; then
  help
elif [ "$command" = "--help" ]; then
  help
else
  echo "Unknown command: $1"
fi
