#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
from pyauditor import AuditorClientBuilder
import argparse
from datetime import datetime
import yaml
import base64
import sys
from auditor_apel_plugin.core import (
    get_token,
    create_db,
    fill_db,
    group_db,
    create_message,
    sign_msg,
    build_payload,
    send_payload,
    get_records,
)
from auditor_apel_plugin.config import get_loaders, Config


def run(config, client, args):
    site = args.site

    message_type = config.plugin.message_type
    field_dict = config.get_all_fields()
    optional_fields = config.get_optional_fields()

    begin_date = datetime.fromisoformat(args.begin_date)
    end_date = datetime.fromisoformat(args.end_date)

    records = get_records(config, client, begin_date, 30, site, end_date)

    if len(records) == 0:
        logging.critical("No records found!")
        sys.exit(1)

    token = get_token(config)
    logging.debug(token)

    db = create_db(field_dict, message_type)
    filled_db = fill_db(config, db, message_type, field_dict, site, records)
    grouped_db = group_db(filled_db, message_type, optional_fields)
    message = create_message(message_type, grouped_db)
    logging.debug(message)
    signed_message = sign_msg(config, message)
    # logging.debug(signed_message)
    encoded_message = base64.b64encode(signed_message).decode("utf-8")
    # logging.debug(encoded_message)
    payload_message = build_payload(encoded_message)
    # logging.debug(payload_message)
    post_message = send_payload(config, token, payload_message)
    logging.debug(post_message.status_code)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--begin_date",
        type=str,
        required=True,
        help="Begin of republishing (UTC): yyyy-mm-dd hh:mm:ss+00:00, "
        "e.g. 2023-11-27 13:31:10+00:00",
    )
    parser.add_argument(
        "--end_date",
        type=str,
        required=True,
        help="End of republishing (UTC): yyyy-mm-dd hh:mm:ss+00:00, "
        "e.g. 2023-11-29 21:10:54+00:00",
    )
    parser.add_argument(
        "-s", "--site", required=True, help="Site (GOCDB): UNI-FREIBURG, ..."
    )
    parser.add_argument("-c", "--config", required=True, help="Path to the config file")
    args = parser.parse_args()

    with open(args.config, "r") as f:
        config: Config = yaml.load(f, Loader=get_loaders())

    log_level = config.plugin.log_level
    log_format = "[%(asctime)s] %(levelname)-8s %(message)s"
    logging.basicConfig(
        level=log_level,
        format=log_format,
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    logging.getLogger("aiosqlite").setLevel("WARNING")
    logging.getLogger("urllib3").setLevel("WARNING")

    auditor_ip = config.auditor.ip
    auditor_port = config.auditor.port
    auditor_timeout = config.auditor.timeout

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
