#!/usr/bin/env bash
set -x
set -eo pipefail

ENV_DIR=${ENV_DIR:=".env_test"}

function make_docs() {
  cd pyauditor
  if [[ -z "${CI_MODE}" ]]
  then
    python -m venv $ENV_DIR
    source $ENV_DIR/bin/activate
    pip install --upgrade pip
  fi
  if [[ -z "${SKIP_COMPILATION}" ]]
  then
    pip install maturin
    SQLX_OFFLINE=true maturin develop
  fi
  pip install sphinx
  pip install sphinx_rtd_theme
  pip install myst-parser
  cd docs
  make html
}

cleanup_exit() {
  setsid nohup bash -c "
    if [[ -z \"${CI_MODE}\" ]]; then rm -rf $ENV_DIR; fi
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

make_docs
