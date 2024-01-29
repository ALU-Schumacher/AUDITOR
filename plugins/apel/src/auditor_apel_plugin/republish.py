#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
from pyauditor import AuditorClientBuilder
import configparser
import argparse
from datetime import datetime, timezone
import base64
import sys
from auditor_apel_plugin.core import (
    get_token,
    create_summary_db,
    group_summary_db,
    create_summary,
    sign_msg,
    build_payload,
    send_payload,
    get_records,
)


def run(config, client, args):
    month = args.month
    year = args.year
    site = args.site

    begin_month = datetime(year, month, 1).replace(tzinfo=timezone.utc)
    if month == 12:
        end_month = datetime(year + 1, 1, 1).replace(tzinfo=timezone.utc)
    else:
        end_month = datetime(year, month + 1, 1).replace(tzinfo=timezone.utc)

    records = get_records(config, client, begin_month, 30, site, end_month)

    if len(records) == 0:
        logging.critical("No records found!")
        sys.exit(1)

    token = get_token(config)
    logging.debug(token)

    summary_db = create_summary_db(config, records)
    grouped_summary_list = group_summary_db(summary_db)
    summary = create_summary(config, grouped_summary_list)
    logging.debug(summary)
    signed_summary = sign_msg(config, summary)
    logging.debug(signed_summary)
    encoded_summary = base64.b64encode(signed_summary).decode("utf-8")
    logging.debug(encoded_summary)
    payload_summary = build_payload(encoded_summary)
    logging.debug(payload_summary)
    post_summary = send_payload(config, token, payload_summary)
    logging.debug(post_summary.status_code)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "-y", "--year", type=int, required=True, help="Year: 2020, 2021, ..."
    )
    parser.add_argument(
        "-m", "--month", type=int, required=True, help="Month: 4, 8, 12, ..."
    )
    parser.add_argument(
        "-s", "--site", required=True, help="Site (GOCDB): UNI-FREIBURG, ..."
    )
    parser.add_argument("-c", "--config", required=True, help="Path to the config file")
    args = parser.parse_args()

    config = configparser.ConfigParser()
    config.read(args.config)

    log_level = config["logging"].get("log_level")
    log_format = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(
        level=log_level,
        format=log_format,
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    logging.getLogger("aiosqlite").setLevel("WARNING")
    logging.getLogger("urllib3").setLevel("WARNING")

    auditor_ip = config["auditor"].get("auditor_ip")
    auditor_port = config["auditor"].getint("auditor_port")
    auditor_timeout = config["auditor"].getint("auditor_timeout")

    builder = AuditorClientBuilder()
    builder = builder.address(auditor_ip, auditor_port).timeout(auditor_timeout)
    client = builder.build_blocking()

    try:
        run(config, client, args)
    except KeyboardInterrupt:
        logging.critical("User abort")
    finally:
        logging.info("Republishing finished")


if __name__ == "__main__":
    main()
