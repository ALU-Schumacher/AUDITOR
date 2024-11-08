#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import argparse
import json
import sqlite3
from datetime import datetime, timezone

import yaml

parser = argparse.ArgumentParser()
parser.add_argument("-c", "--config", required=True, help="Path to the config file")
parser.add_argument("-d", "--db", required=True, help="Path to the time database file")
parser.add_argument("-j", "--json", required=True, help="Path to the time JSON file")
args = parser.parse_args()

with open(args.config) as f:
    config = yaml.safe_load(f)

sites_to_report = config["site"]["sites_to_report"].keys()

db_path = args.db
json_path = args.json

time_db = sqlite3.connect(
    db_path,
    detect_types=sqlite3.PARSE_DECLTYPES | sqlite3.PARSE_COLNAMES,
)

cur = time_db.cursor()
cur.row_factory = lambda cursor, row: row[0]

cur.execute("SELECT last_end_time FROM times")
last_end_time_row = cur.fetchall()
last_end_time = datetime.fromtimestamp(last_end_time_row[0], tz=timezone.utc)

cur.execute("SELECT last_report_time FROM times")
last_report_time_row = cur.fetchall()
last_report_time = last_report_time_row[0]

cur.close()
time_db.close()

time_dict = {
    "last_report_time": last_report_time.isoformat(),
    "site_end_times": {},
}

for site in sites_to_report:
    time_dict["site_end_times"][site] = last_end_time.isoformat()

with open(json_path, "w", encoding="utf-8") as f:
    json.dump(time_dict, f)
