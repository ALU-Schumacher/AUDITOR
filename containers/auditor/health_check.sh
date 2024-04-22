#!/bin/bash

run_health_check() {
  AUDITOR_APPLICATION__PORT="${AUDITOR_APPLICATION__PORT:=8000}"
  curl "localhost:${AUDITOR_APPLICATION__PORT}/health_check" || exit 1
}


if [ $# -eq 0 ]; then
  run_health_check
fi

command="$1"
shift

if [ "$command" = "auditor" ]; then
  run_health_check
else
  exit 0
fi
