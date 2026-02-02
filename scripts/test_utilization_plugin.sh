#!/usr/bin/env bash
set -x
set -eo pipefail

RELEASE_MODE=${RELEASE_MODE:=false}
ENV_DIR=${ENV_DIR:=".env_test"}

function compile_auditor() {
  if [ "$RELEASE_MODE" = true ]; then
    cargo build -p auditor --release
  else
    cargo build -p auditor
  fi
}

function start_auditor() {
  if [[ -z "${SKIP_COMPILATION}" ]]
  then
    compile_auditor
  fi
  if [ "$RELEASE_MODE" = true ]; then
    AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/release/auditor &
  else
    AUDITOR_APPLICATION__ADDR=0.0.0.0 AUDITOR_DATABASE__DATABASE_NAME=$DB_NAME ./target/debug/auditor &
  fi
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
  wait $AUDITOR_SERVER_PID
}

function install_utilization_report_requirements() {
  python -m venv $ENV_DIR
  source $ENV_DIR/bin/activate
  cd plugins/utilization_tool/
  pip install -r requirements.txt
}

function fill_auditor_db() {
  curl -X POST --header "Content-Type: application/json" \
    --data '{"record_id":"record-1","meta":{"site":["bfg"],"VOMS":["/atlas/Role=production/Capability=NULL"],"user":["atlpr001"],"x509DN":["/DC=ch/DC=cern/OU=Organic Units/OU=Users/CN=atlpilo1/CN=614260/CN=Robot: ATLAS Pilot1"]},"components":[{"name":"Cores","amount":8,"scores":[{"name":"HEPscore23","value":18.4}]},{"name":"RequestedMemory","amount":16000,"scores":[]},{"name":"UsedMemory","amount":13121500,"scores":[]},{"name":"TotalCPU","amount":324976,"scores":[]},{"name":"DiskUsage","amount":5923,"scores":[]},{"name":"BytesIn","amount":1736736,"scores":[]},{"name":"BytesOut","amount":4428425,"scores":[]}],"start_time":"2024-11-18T12:08:46Z","stop_time":"2024-11-18T23:14:50Z","runtime":39964}' \
    http://localhost:8000/record

  curl -X POST --header "Content-Type: application/json" \
    --data '{"record_id":"record-2","meta":{"site":["bfg"],"VOMS":["/atlas/Role=production/Capability=NULL"],"user":["atlpr001"]},"components":[{"name":"Cores","amount":8,"scores":[{"name":"HEPscore23","value":18.4}]},{"name":"RequestedMemory","amount":16000,"scores":[]},{"name":"UsedMemory","amount":13121500,"scores":[]},{"name":"TotalCPU","amount":324976,"scores":[]},{"name":"DiskUsage","amount":5923,"scores":[]},{"name":"BytesIn","amount":1736736,"scores":[]},{"name":"BytesOut","amount":4428425,"scores":[]}],"start_time":"2024-11-28T12:08:46Z","stop_time":"2024-11-28T23:14:50Z","runtime":39964}' \
    http://localhost:8000/record

  curl -X POST --header "Content-Type: application/json" \
    --data '{"record_id":"record-3","meta":{"CPUUsage":["1.248667303247056"],"user":["atlpr001"],"site":["bfg"],"VOMS":["/atlas/Role=ilc/Capability=NULL"]},"components":[{"name":"Cores","amount":1,"scores":[{"name":"HEPscore23","value":18.4}]},{"name":"RequestedMemory","amount":3000,"scores":[]},{"name":"UsedMemory","amount":2130012,"scores":[]},{"name":"TotalCPU","amount":11042,"scores":[]},{"name":"DiskUsage","amount":318574,"scores":[]},{"name":"BytesIn","amount":3475363,"scores":[]},{"name":"BytesOut","amount":1688048,"scores":[]}],"start_time":"2024-11-21T10:36:13Z","stop_time":"2024-11-21T13:10:32Z","runtime":9259}' \
    http://localhost:8000/record
  
  curl -X POST --header "Content-Type: application/json" \
    --data '{"record_id":"record-4","meta":{"CPUUsage":["1.248667303247056"],"user":["atlpr001"],"site":["bfg"],"VOMS":["/atlas/Role=ilc/Capability=NULL"]},"components":[{"name":"Cores","amount":1,"scores":[{"name":"HEPscore23","value":18.4}]},{"name":"RequestedMemory","amount":3000,"scores":[]},{"name":"UsedMemory","amount":2130012,"scores":[]},{"name":"TotalCPU","amount":11042,"scores":[]},{"name":"DiskUsage","amount":318574,"scores":[]},{"name":"BytesIn","amount":3475363,"scores":[]},{"name":"BytesOut","amount":1688048,"scores":[]}],"start_time":"2024-11-21T10:36:13Z","stop_time":"2024-11-21T13:10:32Z","runtime":9259}' \
    http://localhost:8000/record
  
}

function start_utilization_plugin {
  python3 src/auditor_utilization_plugin/main.py -c configs/config.yaml --month 11 --year 2024 --oneshot True
}

function validate_summary_report {
  row1="production,3.268167111111111,1.0164648183365028,0.17761777777777776,0.8170417777777778,0.2965861653333333"
  row2="ilc,0.09464755555555554,1.1925693919429745,0.005143888888888889,0.023661888888888885,0.008589265666666665"


  if grep -Fxq "$row1" summary_11_2024.csv; then
    echo "test data Row1 is computed correctly"
  else
    {
    echo "test data row1 NOT found or computed summary is wrong"
    exit 1
  }
  fi

  if grep -Fxq "$row2" summary_11_2024.csv; then
    echo "test data Row2 is computed correctly"
  else
    {
    echo "test data row2 NOT found or computed summary is wrong"
    exit 1
  }
  fi
}

cleanup_exit() {
  setsid nohup bash -c "
  kill $AUDITOR_SERVER_PID
  if [[ -z \"${SKIP_PYAUDITOR_COMPILATION}\" ]]; then rm -rf $ENV_DIR; fi
  "
}
trap "cleanup_exit" SIGINT SIGQUIT SIGTERM EXIT


start_auditor

fill_auditor_db

install_utilization_report_requirements

start_utilization_plugin

validate_summary_report

stop_auditor
