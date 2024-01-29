#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
from pyauditor import AuditorClientBuilder
from datetime import datetime, timedelta, timezone
import configparser
import argparse
import base64
from time import sleep
from auditor_apel_plugin.core import (
    get_token,
    get_time_db,
    get_report_time,
    get_start_time,
    create_summary_db,
    group_summary_db,
    create_summary,
    sign_msg,
    build_payload,
    send_payload,
    update_time_db,
    get_begin_previous_month,
    create_sync_db,
    group_sync_db,
    create_sync,
    get_records,
    check_sites_in_records,
)


def run(config, client):
    report_interval = config["intervals"].getint("report_interval")
    token = get_token(config)
    logging.debug(token)

    while True:
        time_db_conn = get_time_db(config)
        last_report_time = get_report_time(time_db_conn)
        current_time = datetime.now()
        time_since_report = (current_time - last_report_time).total_seconds()

        if time_since_report < report_interval:
            logging.info("Not enough time since last report")
            time_db_conn.close()
            sleep(report_interval - time_since_report)
            continue
        else:
            logging.info("Enough time since last report, create new report")

        start_time = get_start_time(time_db_conn)
        logging.info(f"Getting records since {start_time}")

        records_summary = get_records(config, client, start_time, 30)

        if len(records_summary) == 0:
            logging.info("No new records, do nothing for now")
            time_db_conn.close()
            logging.info(
                "Next report scheduled for "
                f"{datetime.now() + timedelta(seconds=report_interval)}"
            )
            sleep(report_interval)
            continue

        sites_to_report = check_sites_in_records(config, records_summary)
        logging.info(f"Create reports for {sites_to_report}")

        latest_stop_time = records_summary[-1].stop_time.replace(tzinfo=timezone.utc)

        logging.debug(f"Latest stop time is {latest_stop_time}")
        summary_db = create_summary_db(config, records_summary)
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

        begin_previous_month = get_begin_previous_month(current_time)
        records_sync = get_records(config, client, begin_previous_month, 30)
        sync_db = create_sync_db(config, records_sync)
        grouped_sync_list = group_sync_db(sync_db)
        sync = create_sync(grouped_sync_list)
        logging.debug(sync)
        signed_sync = sign_msg(config, sync)
        logging.debug(signed_sync)
        encoded_sync = base64.b64encode(signed_sync).decode("utf-8")
        logging.debug(encoded_sync)
        payload_sync = build_payload(encoded_sync)
        logging.debug(payload_sync)
        post_sync = send_payload(config, token, payload_sync)
        logging.debug(post_sync.status_code)

        latest_report_time = datetime.now()
        update_time_db(time_db_conn, latest_stop_time.timestamp(), latest_report_time)

        time_db_conn.close()
        logging.info(
            "Next report scheduled for "
            f"{datetime.now() + timedelta(seconds=report_interval)}"
        )
        sleep(report_interval)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-c", "--config", required=True, help="Path to the config file")
    args = parser.parse_args()

    config = configparser.ConfigParser()
    config.read(args.config)

    log_level = config["logging"].get("log_level")
    log_format = (
        "[%(asctime)s] %(levelname)-8s %(message)s (%(pathname)s at line %(lineno)d)"
    )
    logging.basicConfig(
        # filename="apel_plugin.log",
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
        run(config, client)
    except KeyboardInterrupt:
        logging.critical("User abort")
    finally:
        logging.critical("APEL plugin stopped")


if __name__ == "__main__":
    main()
