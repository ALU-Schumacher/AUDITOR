#!/usr/bin/env sh

#!/bin/bash
set -euo pipefail

APPDIR=/opt/auditor_htcondor_collector
VENV=$APPDIR/venv

if [ ! -x "$VENV/bin/python" ]; then
  python3 -m venv "$VENV"
  "$VENV/bin/python" -m pip install --upgrade pip setuptools wheel
  "$VENV/bin/pip" install auditor_htcondor_collector=="$1"
fi
