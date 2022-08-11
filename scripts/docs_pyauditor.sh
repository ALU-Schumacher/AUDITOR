#!/usr/bin/env bash
set -x
set -eo pipefail

ENV_DIR=${ENV_DIR:=".env_test"}

function make_docs() {
  cd pyauditor
	python -m venv $ENV_DIR
	source $ENV_DIR/bin/activate
  pip install maturin
  pip install sphinx
  pip install sphinx_rtd_theme
  SQLX_OFFLINE=true maturin develop
  cd docs
  make html
}

cleanup_exit() {
  setsid nohup bash -c "
    rm -rf $ENV_DIR
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT

make_docs
