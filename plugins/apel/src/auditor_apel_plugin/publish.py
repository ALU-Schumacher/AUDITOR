#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import argparse
import logging
from datetime import datetime, timedelta, timezone
from logging import Logger
from logging.handlers import RotatingFileHandler
from time import sleep

import yaml
from pyauditor import AuditorClientBuilder

from auditor_apel_plugin.config import Config, MessageType, get_loaders
from auditor_apel_plugin.core import (
    build_payload,
    create_db,
    create_message,
    fill_db,
    get_begin_current_month,
    get_begin_previous_month,
    get_records,
    get_report_time,
    get_start_time,
    get_time_json,
    get_total_numbers,
    group_db,
    send_payload,
    sign_msg,
    update_time_json,
)

TRACE = logging.DEBUG - 5


def run(logger: Logger, config: Config, client, args):
    report_interval = config.plugin.report_interval
    sites_to_report = config.site.sites_to_report
    message_type = config.plugin.message_type
    field_dict = config.get_all_fields()
    optional_fields = config.get_optional_fields()
    dry_run = args.dry_run

    if dry_run:
        logger.info("Starting one-shot dry-run, nothing will be sent to APEL!")

    while True:
        time_dict = get_time_json(config)
        last_report_time = get_report_time(time_dict)
        current_time = datetime.now()
        time_since_report = (current_time - last_report_time).total_seconds()

        if not dry_run:
            if time_since_report < report_interval:
                logger.info("Not enough time since last report")
                logger.info(f"Next report scheduled for {next_report_time}")
                sleep(report_interval - time_since_report)
                continue
            else:
                logger.info("Enough time since last report, create new report")

        if current_time.day < last_report_time.day:
            begin_month = get_begin_previous_month(current_time)
        else:
            begin_month = get_begin_current_month(current_time)

        for site in sites_to_report.keys():
            records = get_records(config, client, begin_month, 30, site=site)

            if len(records) == 0:
                logger.info(f"No new records for {site}")
                continue

            latest_stop_time = records[-1].stop_time.replace(tzinfo=timezone.utc)
            logger.debug(f"Latest stop time is {latest_stop_time}")

            sync_db = create_db({}, MessageType.sync)
            filled_sync_db = fill_db(
                config, sync_db, MessageType.sync, {}, site, records
            )
            grouped_sync_db = group_db(filled_sync_db, MessageType.sync, {})
            filled_sync_db.close()
            sync_message = create_message(MessageType.sync, grouped_sync_db)
            logger.debug(f"Sync message:\n{sync_message}")
            signed_sync = sign_msg(config, sync_message)
            logger.debug(f"Signed sync message:\n{signed_sync}")
            payload_sync = build_payload(signed_sync)

            logger.debug(f"Payload sync message:\n{payload_sync}")
            if not dry_run:
                post_sync = send_payload(config, payload_sync)
                logger.info(f"Sync message sent to server, response:\n{post_sync}")

            if message_type == MessageType.individual_jobs:
                start_time = get_start_time(config, time_dict, site)
                logger.info(f"Getting records since {start_time}")
                records = [
                    r
                    for r in records
                    if r.stop_time.replace(tzinfo=timezone.utc) > start_time
                ]

                if len(records) == 0:
                    logger.info(f"No new records for {site}")
                    continue

            db = create_db(field_dict, message_type)
            filled_db = fill_db(config, db, message_type, field_dict, site, records)
            del records
            grouped_db = group_db(filled_db, message_type, optional_fields)
            message = create_message(message_type, grouped_db)
            logger.log(TRACE, f"Message:\n{message}")
            signed_message = sign_msg(config, message)
            logger.log(TRACE, f"Signed message:\n{signed_message}")
            payload_message = build_payload(signed_message)

            logger.log(TRACE, f"Payload message:\n{payload_message}")
            if not dry_run:
                post_message = send_payload(config, payload_message)
                logger.info(f"Message sent to server, response:\n{post_message}")

                latest_report_time: datetime = datetime.now()
                update_time_json(
                    config, time_dict, site, latest_stop_time, latest_report_time
                )

            total_numbers = get_total_numbers(filled_db)
            filled_db.close()
            logger.info(f"Total numbers reported by the plugin:\n{total_numbers}")

        if dry_run:
            logger.info("One-shot dry-run finished!")
            quit()

        next_report_time: datetime = latest_report_time + timedelta(
            seconds=report_interval
        )
        logger.info(f"Next report scheduled for {next_report_time}")

        sleep(report_interval)


def main():
    parser = argparse.ArgumentParser()
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
    log_file = config.plugin.log_file
    log_format = (
        "[%(asctime)s] %(levelname)-8s %(message)s (%(pathname)s at line %(lineno)d)"
    )
    date_format = "%Y-%m-%d %H:%M:%S"

    logging.addLevelName(TRACE, "TRACE")
    logging.basicConfig(
        level=log_level,
        format=log_format,
        datefmt=date_format,
    )
    logging.getLogger("aiosqlite").setLevel("WARNING")
    logging.getLogger("urllib3").setLevel("WARNING")

    logger = logging.getLogger("apel_plugin")

    if log_file is not None:
        handler = RotatingFileHandler(
            log_file,
            maxBytes=10 * 1024 * 1024,
            backupCount=5,
        )
        handler.setFormatter(logging.Formatter(log_format, date_format))
        logger.addHandler(handler)

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
        logger.critical("APEL plugin stopped")


if __name__ == "__main__":
    main()
