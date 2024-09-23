#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
import sqlite3
from sqlite3 import Error
from datetime import datetime, timedelta, time, timezone
from time import sleep
import json
import sys
import requests
import urllib
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.serialization import pkcs7
from pyauditor import Value, Operator, MetaOperator, MetaQuery, QueryBuilder, Record
from auditor_apel_plugin.config import Field, MessageType, Config
from auditor_apel_plugin.message import (
    SummaryMessage,
    SyncMessage,
    SingleJobMessage,
    Message,
)
from typing import Dict, List, Tuple

logger = logging.getLogger("apel_plugin")


def get_records(
    config: Config, client, start_time, delay_time, site=None, end_time=None
):
    sites_to_report = config.site.sites_to_report
    meta_key_site = config.auditor.site_meta_field

    site_ids = []

    if site is not None:
        site_ids = sites_to_report[site]
        logger.info(f"Getting records for site {site} with site_ids: {site_ids}")
    else:
        for k, v in sites_to_report.items():
            site_ids.extend(v)

        logger.info(
            f"Getting records for sites {list(sites_to_report.keys())} "
            f"with site_ids: {list(sites_to_report.values())}"
        )

    timeout_counter = 0
    records = []

    while timeout_counter < 2:
        try:
            start_time_value = Value.set_datetime(start_time)
            get_since_operator = Operator().gt(start_time_value)
            stop_time_query = QueryBuilder().with_stop_time(get_since_operator)
            if end_time is not None:
                end_time_value = Value.set_datetime(end_time)
                get_range_operator = get_since_operator.lt(end_time_value)
                stop_time_query = stop_time_query.with_stop_time(get_range_operator)
            for site in site_ids:
                site_operator = MetaOperator().contains(site)
                site_query = MetaQuery().meta_operator(meta_key_site, site_operator)
                query_string = stop_time_query.with_meta_query(site_query).build()
                records.extend(client.advanced_query(query_string))
            return records
        except Exception as e:
            if "timed" in str(e):
                timeout_counter += 1
                logger.warning(
                    f"Call to AUDITOR timed out {timeout_counter}/3! "
                    f"Trying again in {timeout_counter * delay_time}s"
                )
                sleep(timeout_counter * delay_time)
            else:
                logger.critical(e)
                raise

    logger.critical(
        "Call to AUDITOR timed out 3/3, quitting! "
        "Maybe increase auditor_timeout in the config"
    )

    sys.exit(1)


def get_begin_previous_month(current_time):
    first_current_month = current_time.replace(day=1)
    previous_month = first_current_month - timedelta(days=1)
    first_previous_month = previous_month.replace(day=1)
    begin_previous_month = datetime.combine(first_previous_month, time())
    begin_previous_month_utc = begin_previous_month.replace(tzinfo=timezone.utc)

    return begin_previous_month_utc


def get_begin_current_month(current_time):
    first_current_month = current_time.replace(day=1)
    begin_current_month = datetime.combine(first_current_month, time())
    begin_current_month_utc = begin_current_month.replace(tzinfo=timezone.utc)

    return begin_current_month_utc


def get_time_json(config):
    time_json_path = config.plugin.time_json_path

    try:
        with open(time_json_path, "r", encoding="utf-8") as f:
            time_dict = json.load(f)
    except FileNotFoundError:
        logger.warning(f"Path {time_json_path} not found, creating new time json")
        time_dict = create_time_json(time_json_path)

    return time_dict


def create_time_json(time_json_path):
    initial_report_time = datetime(1970, 1, 1, 0, 0, 0)
    time_dict = {
        "last_report_time": initial_report_time.isoformat(),
        "site_end_times": {},
    }

    try:
        with open(time_json_path, "w", encoding="utf-8") as f:
            json.dump(time_dict, f)
    except FileNotFoundError:
        logger.critical(f"Path {time_json_path} not found, could not create time json")
        raise

    return time_dict


def get_start_time(config, time_dict, site):
    try:
        start_time = datetime.fromisoformat(time_dict["site_end_times"][site])
    except KeyError:
        start_time = config.site.publish_since

    return start_time


def get_report_time(time_dict):
    report_time = datetime.fromisoformat(time_dict["last_report_time"])

    return report_time


def update_time_json(config, time_dict, site, stop_time, report_time):
    time_json_path = config.plugin.time_json_path

    time_dict["last_report_time"] = report_time.isoformat()
    time_dict["site_end_times"][site] = stop_time.isoformat()

    try:
        with open(time_json_path, "w", encoding="utf-8") as f:
            json.dump(time_dict, f)
    except FileNotFoundError:
        logger.critical(f"Path {time_json_path} not found, could not update time json")
        raise


def replace_record_string(string):
    updated_string = urllib.parse.unquote(string)

    return updated_string


def get_site_id(config, record):
    meta_key_site = config["auditor"]["meta_key_site"]

    try:
        site_id = record.meta.get(meta_key_site)[0]
        return site_id
    except AttributeError:
        logger.critical(f"No meta data found in {record.record_id}, aborting")
        raise
    except TypeError:
        logger.critical(f"No site name found in {record.record_id}, aborting")
        raise


def get_voms_info(config, record):
    meta_key_voms = config["auditor"]["meta_key_voms"]
    voms_dict = {}

    try:
        voms_string = replace_record_string(record.meta.get(meta_key_voms)[0])
    except TypeError:
        logger.warning(
            f"No VOMS information found in {record.record_id}, "
            "not sending VO, VOGroup, and VORole"
        )

        voms_dict["vo"] = None
        voms_dict["vogroup"] = None
        voms_dict["vorole"] = None

        return voms_dict

    if not voms_string.startswith("/"):
        logger.warning(
            f"VOMS information found in {record.record_id} has unknown "
            f"format: {voms_string}. Not sending VO, VOGroup, and VORole"
        )

        voms_dict["vo"] = None
        voms_dict["vogroup"] = None
        voms_dict["vorole"] = None

        return voms_dict

    voms_list = voms_string.split("/")
    voms_dict["vo"] = voms_list[1]

    if "Role" not in voms_string:
        logger.warning(
            f"No Role found in VOMS of {record.record_id}: {voms_string}, "
            "not sending VORole"
        )
        voms_dict["vorole"] = None

        if len(voms_list) == 2:
            voms_dict["vogroup"] = "/" + voms_list[1]
        else:
            voms_dict["vogroup"] = "/" + voms_list[1] + "/" + voms_list[2]
    elif len(voms_list) == 3:
        voms_dict["vogroup"] = "/" + voms_list[1]
        voms_dict["vorole"] = voms_list[2]
    else:
        voms_dict["vogroup"] = "/" + voms_list[1] + "/" + voms_list[2]
        voms_dict["vorole"] = voms_list[3]

    return voms_dict


def create_db(
    fields_dict: Dict[str, Field], message_type: MessageType
) -> sqlite3.Connection:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.individual_jobs:
        message = SingleJobMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    field_list = message.create_sql

    for k, v in fields_dict.items():
        if k not in message.message_fields:
            logger.critical(
                f"Field {k} not in list of possible fields: {message.message_fields}"
            )
            raise ValueError
        else:
            field_list.append(f"{k} NOT NULL")

    field_list_str = ",".join(field_list)

    create_db_str = "".join(
        ["CREATE TABLE IF NOT EXISTS records(", field_list_str, ")"]
    )

    conn = sqlite3.connect(":memory:")

    try:
        with conn:
            conn.execute(create_db_str)
    except Error as e:
        logger.critical(e)
        raise

    return conn


def fill_db(
    config: Config,
    conn: sqlite3.Connection,
    message_type: MessageType,
    fields_dict: Dict[str, Field],
    site: str,
    records: List[Record],
) -> sqlite3.Connection:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.individual_jobs:
        message = SingleJobMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    field_list = [field.split(" ")[0] for field in message.create_sql]

    for k in fields_dict.keys():
        field_list.append(k)

    field_list_str = ",".join(field_list)

    q_marks = ",".join(len(field_list) * ["?"])

    insert_db_str = "".join(
        ["INSERT INTO records(", field_list_str, ") VALUES(", q_marks, ")"]
    )

    for r in records:
        data_tuple = get_data_tuple(config, message_type, fields_dict, site, r)

        try:
            with conn:
                conn.execute(insert_db_str, data_tuple)
        except Error as e:
            logger.critical(e)
            raise

    return conn


def get_data_tuple(
    config: Config,
    message_type: MessageType,
    fields_dict: Dict[str, Field],
    site: str,
    record: Record,
) -> Tuple[int, float, str]:
    value_list = []

    if message_type == MessageType.summaries:
        month = record.stop_time.replace(tzinfo=timezone.utc).month
        year = record.stop_time.replace(tzinfo=timezone.utc).year
        stop_time = record.stop_time.replace(tzinfo=timezone.utc).timestamp()
        runtime = record.runtime
        record_id = record.record_id

        value_list = [site, month, year, stop_time, runtime, record_id]

    elif message_type == MessageType.individual_jobs:
        record_id = record.record_id
        runtime = record.runtime
        start_time = record.start_time.replace(tzinfo=timezone.utc).timestamp()
        stop_time = record.stop_time.replace(tzinfo=timezone.utc).timestamp()

        value_list = [site, record_id, runtime, start_time, stop_time]

    elif message_type == MessageType.sync:
        month = record.stop_time.replace(tzinfo=timezone.utc).month
        year = record.stop_time.replace(tzinfo=timezone.utc).year
        submithost_field = config.get_optional_fields().get("SubmitHost")
        if submithost_field is not None:
            submithost = replace_record_string(submithost_field.get_value(record))
        else:
            submithost = "None"

        record_id = record.record_id

        value_list = [site, month, year, submithost, record_id]

    for v in fields_dict.values():
        value = v.get_value(record)

        if isinstance(value, str):
            value = replace_record_string(value)

        value_list.append(value)

    data_tuple = tuple(value_list)

    return data_tuple


def group_db(
    conn: sqlite3.Connection, message_type: MessageType, fields_dict: Dict[str, Field]
) -> List[sqlite3.Row]:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.individual_jobs:
        message = SingleJobMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    group_by_list = message.group_by

    for k in fields_dict.keys():
        group_by_list.append(k)

    sql_group_by = ",".join(group_by_list)
    sql_store_as = ",".join(message.store_as)

    group_str = "".join(
        [
            "SELECT ",
            sql_group_by,
            ",",
            sql_store_as,
            " FROM records GROUP BY ",
            sql_group_by,
        ]
    )

    conn.row_factory = sqlite3.Row
    cur = conn.execute(group_str)
    grouped_sql = cur.fetchall()
    cur.close()
    conn.close()

    return grouped_sql


def create_message(message_type: MessageType, grouped_sql: List[sqlite3.Row]) -> str:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.individual_jobs:
        message = SingleJobMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    header = message.message_header
    message_fields = message.message_fields

    field_list = [header]

    for entry in grouped_sql:
        keys = entry.keys()

        for field in message_fields:
            if field in keys:
                field_list.append(f"{field}: {entry[field]}\n")
            else:
                field_list.append(f"{field}: None\n")

        field_list.append("%%\n")

    apel_message = "".join(field_list)

    return apel_message


def get_token(config):
    auth_url = config.authentication.auth_url
    client_cert = config.authentication.client_cert
    client_key = config.authentication.client_key
    verify_ca = config.authentication.verify_ca

    if verify_ca:
        ca_path = config.authentication.ca_path
    else:
        ca_path = False

    try:
        response = requests.get(
            auth_url, cert=(client_cert, client_key), verify=ca_path, timeout=10
        )
    except requests.Timeout:
        logger.critical("Timeout while getting token")
        raise

    token = response.json()["token"]

    return token


def sign_msg(config, msg):
    client_cert = config.authentication.client_cert
    client_key = config.authentication.client_key

    with open(client_cert, "rb") as cc:
        cert = x509.load_pem_x509_certificate(cc.read())

    with open(client_key, "rb") as ck:
        key = serialization.load_pem_private_key(ck.read(), None)

    options = [pkcs7.PKCS7Options.DetachedSignature, pkcs7.PKCS7Options.Text]

    signed_msg = (
        pkcs7.PKCS7SignatureBuilder()
        .set_data(bytes(msg, "utf-8"))
        .add_signer(cert, key, hashes.SHA256())
        .sign(serialization.Encoding.SMIME, options)
    )

    return signed_msg


def build_payload(msg):
    current_time = datetime.now(timezone.utc).strftime("%Y%m%d%H%M%S")
    empaid = f"{current_time[:8]}/{current_time}"

    payload = {"messages": [{"attributes": {"empaid": empaid}, "data": msg}]}

    return payload


def send_payload(config, token, payload):
    ams_url = config.authentication.ams_url
    verify_ca = config.authentication.verify_ca

    if verify_ca:
        ca_path = config.authentication.ca_path
    else:
        ca_path = False

    logger.debug(f"{ams_url}{token}")
    post = requests.post(
        f"{ams_url}{token}",
        json=payload,
        headers={"Content-Type": "application/json"},
        verify=ca_path,
    )

    return post


def convert_to_seconds(config, cpu_time):
    cpu_time_name = config["auditor"]["cpu_time_name"]
    cpu_time_unit = config["auditor"]["cpu_time_unit"]

    if cpu_time_unit == "seconds":
        return cpu_time
    elif cpu_time_unit == "milliseconds":
        return round(cpu_time / 1000)
    else:
        logger.critical(
            f"Unknown unit for {cpu_time_name}: {cpu_time_unit}. "
            "Possible values are seconds or milliseconds."
        )
        raise ValueError


def check_sites_in_records(config, records):
    sites_to_report = config["site"]["sites_to_report"]

    logger.debug(f"Sites to report from config: {list(sites_to_report.keys())}")

    sites_in_records = {get_site_id(config, r) for r in records}
    sites = []

    for site_id in sites_in_records:
        for k, v in sites_to_report.items():
            if site_id in v:
                sites.append(k)
                break

    logger.debug(f"Sites found in records: {sites}")

    return sites
