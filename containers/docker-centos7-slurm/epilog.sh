#!/usr/bin/env bash
set -x
set -eo pipefail

curl http://localhost:8000/health_check
