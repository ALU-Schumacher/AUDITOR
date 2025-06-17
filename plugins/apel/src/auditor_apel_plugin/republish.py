#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import argparse
import logging
from datetime import datetime, timedelta, timezone
from logging import Logger
from typing import Dict, Union

import yaml
from pyauditor import AuditorClientBuilder

from auditor_apel_plugin.config import Config, MessageType, get_loaders
from auditor_apel_plugin.core import (
    build_payload,
    create_db,
    create_dict,
    create_message,
    fill_db,
    get_records,
    group_db,
    send_payload,
    sign_msg,
)

TRACE = logging.DEBUG - 5


def run(logger: Logger, config: Config, client, args):
    site = args.site
    dry_run = args.dry_run

    sites_to_report = config.site.sites_to_report
    field_dict = config.get_all_fields()
    optional_fields = config.get_optional_fields()

    month = args.month
    year = args.year

    has_records = False

    begin_month = datetime(year, month, 1, tzinfo=timezone.utc)
    if month == 12:
        end_month = datetime(year + 1, 1, 1, tzinfo=timezone.utc)
    else:
        end_month = datetime(year, month + 1, 1, tzinfo=timezone.utc)

    if dry_run:
        logger.info("Starting one-shot dry-run, nothing will be sent to APEL!")

    aggr_summary_dict: Dict[str, Dict[str, Union[str, int]]] = {}
    loop_day = begin_month

    while end_month > loop_day:
        next_day = loop_day + timedelta(days=1)

        logger.info(
            f"Getting records for {loop_day.date()} for site {site} "
            f"with site_ids: {sites_to_report[site]}"
        )

        if next_day > end_month:
            next_day = end_month

        records = get_records(config, client, loop_day, site, next_day)

        loop_day = next_day

        if len(records) == 0:
            logger.warning(f"No records for {site} on this day")
            continue

        has_records = True

        latest_stop_time = records[-1].stop_time.replace(tzinfo=timezone.utc)
        logger.debug(f"Latest stop time is {latest_stop_time}")

        db = create_db(field_dict, MessageType.summaries)
        filled_db = fill_db(
            config,
            db,
            MessageType.summaries,
            field_dict,
            site,
            records,
        )
        del records
        grouped_db = group_db(filled_db, MessageType.summaries, optional_fields)
        filled_db.close()
        message_dict = create_dict(
            MessageType.summaries, grouped_db, optional_fields, aggr_summary_dict
        )

    if not has_records:
        logger.warning(f"No records for site {site} in this month")
        quit()

    message = create_message(MessageType.summaries, message_dict)
    logger.log(TRACE, f"Message:\n{message}")
    signed_message = sign_msg(config, message)
    logger.log(TRACE, f"Signed message:\n{signed_message}")
    payload_message = build_payload(signed_message)
    logger.log(TRACE, f"Payload message:\n{payload_message}")

    if not dry_run:
        post_message = send_payload(config, payload_message)
        logger.info(f"Message sent to server, response:\n{post_message}")


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
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="One-shot dry-run, nothing will be sent to the APEL server",
    )
    args = parser.parse_args()

    with open(args.config, "r") as f:
        config: Config = yaml.load(f, Loader=get_loaders())

    log_level = config.plugin.log_level
    log_format = (
        "[%(asctime)s] %(levelname)-8s %(message)s (%(pathname)s at line %(lineno)d)"
    )

    logging.addLevelName(TRACE, "TRACE")
    logging.basicConfig(
        level=log_level,
        format=log_format,
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    logging.getLogger("aiosqlite").setLevel("WARNING")
    logging.getLogger("urllib3").setLevel("WARNING")

    logger = logging.getLogger("apel_plugin")

    auditor_ip = config.auditor.ip
    auditor_port = config.auditor.port
    auditor_timeout = config.auditor.timeout
    auditor_tls = config.auditor.use_tls

    builder = AuditorClientBuilder()

    if auditor_tls:
        auditor_ca_cert = config.auditor.ca_cert_path
        auditor_client_cert = config.auditor.client_cert_path
        auditor_client_key = config.auditor.client_key_path
        builder = builder.with_tls(
            auditor_client_cert, auditor_client_key, auditor_ca_cert
        )

    builder = builder.address(auditor_ip, auditor_port).timeout(auditor_timeout)
    client = builder.build_blocking()

    try:
        run(logger, config, client, args)
    except KeyboardInterrupt:
        logger.critical("User abort")
    finally:
        if args.dry_run:
            logger.info("One-shot dry-run finished!")
        else:
            logger.info("Republishing finished")


if __name__ == "__main__":
    main()
