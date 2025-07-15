#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2022 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import hashlib
import json
import logging
import sqlite3
from datetime import datetime, time, timedelta, timezone
from sqlite3 import Error
from typing import Dict, List, Tuple, Union, cast

from argo_ams_library import AmsException, AmsMessage, ArgoMessagingService
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.serialization import pkcs7
from pyauditor import MetaOperator, MetaQuery, Operator, QueryBuilder, Record, Value

from auditor_apel_plugin.config import Config, Field, MessageType
from auditor_apel_plugin.message import (
    Message,
    PluginMessage,
    SummaryMessage,
    SyncMessage,
)

from .utility import write_transaction

logger = logging.getLogger("apel_plugin")


def get_records(config: Config, client, start_time, site=None, end_time=None):
    sites_to_report = config.site.sites_to_report
    meta_key_site = config.auditor.site_meta_field

    if isinstance(meta_key_site, str):
        meta_key_site = [meta_key_site]

    site_ids = []

    if site is not None:
        site_ids = sites_to_report[site]
    else:
        for v in sites_to_report.values():
            site_ids.extend(v)

    records = []

    try:
        start_time_value = Value.set_datetime(start_time)
        get_since_operator = Operator().gt(start_time_value)
        stop_time_query = QueryBuilder().with_stop_time(get_since_operator)
        if end_time is not None:
            end_time_value = Value.set_datetime(end_time)
            get_range_operator = get_since_operator.lt(end_time_value)
            stop_time_query = stop_time_query.with_stop_time(get_range_operator)
        for site in site_ids:
            site_operator = MetaOperator().contains([site])
            for meta_key in meta_key_site:
                site_query = MetaQuery().meta_operator(meta_key, site_operator)
                query_string = stop_time_query.with_meta_query(site_query).build()
                records.extend(client.advanced_query(query_string))
        return records
    except Exception as e:
        logger.critical(e)
        raise


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
        with write_transaction(time_json_path, encoding="utf-8") as f:
            json.dump(time_dict, f)
    except FileNotFoundError:
        logger.critical(f"Path {time_json_path} not found, could not create time json")
        raise

    return time_dict


def get_report_time(time_dict):
    report_time = datetime.fromisoformat(time_dict["last_report_time"])

    return report_time


def update_time_json(config, time_dict, site, stop_time, report_time):
    time_json_path = config.plugin.time_json_path

    time_dict["last_report_time"] = report_time.isoformat()
    time_dict["site_end_times"][site] = stop_time.isoformat()

    try:
        with write_transaction(time_json_path, encoding="utf-8") as f:
            json.dump(time_dict, f)
    except FileNotFoundError:
        logger.critical(f"Path {time_json_path} not found, could not update time json")
        raise


def create_db(
    fields_dict: Dict[str, Field], message_type: MessageType
) -> sqlite3.Connection:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
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

    elif message_type == MessageType.sync:
        month = record.stop_time.replace(tzinfo=timezone.utc).month
        year = record.stop_time.replace(tzinfo=timezone.utc).year
        submithost_field = config.get_mandatory_fields().get("SubmitHost")
        if submithost_field is not None:
            submithost = submithost_field.get_value(record)
        else:
            logger.warning("SubmitHost field not defined!")
            submithost = "None"

        record_id = record.record_id

        value_list = [site, month, year, submithost, record_id]

    for v in fields_dict.values():
        value = v.get_value(record)
        value_list.append(value)

    data_tuple = tuple(value_list)

    return data_tuple


def group_db(
    conn: sqlite3.Connection, message_type: MessageType, fields_dict: Dict[str, Field]
) -> List[sqlite3.Row]:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
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

    return grouped_sql


def get_total_numbers(summary_dict: Dict[str, Dict[str, Union[str, int]]]) -> str:
    message = PluginMessage()
    aggr_fields = message.aggr_fields
    group_by_list = message.group_by

    hash_dict: Dict[str, Dict[str, Union[str, int]]] = {}

    for group in summary_dict.values():
        hashstr = ""
        message_dict = {}

        for k, v in group.items():
            if k in group_by_list:
                hashstr += str(v)
                message_dict[k] = v
            if k in aggr_fields:
                message_dict[k] = v

        hash_value = hashlib.md5(hashstr.encode()).hexdigest()

        if hash_value in hash_dict:
            aggr_numbers = aggregate_messages(
                message_dict, hash_dict[hash_value], aggr_fields
            )
            hash_dict[hash_value] = aggr_numbers
        else:
            hash_dict[hash_value] = message_dict

    total_numbers = []

    for group in hash_dict.values():
        for k, v in group.items():
            total_numbers.append(f"{k}: {v}\n")

    total_numbers_message = "".join(total_numbers)

    return total_numbers_message


def create_dict(
    message_type: MessageType,
    grouped_sql: List[sqlite3.Row],
    fields_dict: Dict[str, Field],
    hash_dict: Dict[str, Dict[str, Union[str, int]]],
) -> Dict[str, Dict[str, Union[str, int]]]:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    group_by_list = message.group_by
    for k in fields_dict.keys():
        group_by_list.append(k)

    message_fields = message.message_fields
    aggr_fields = message.aggr_fields

    for entry in grouped_sql:
        message_dict = {}
        hashstr = ""
        keys = entry.keys()

        for field in message_fields:
            if field in keys:
                message_dict[field] = entry[field]
            else:
                logger.debug(f"Field {field} not defined in config, skipping")
            if field in group_by_list:
                hashstr += str(entry[field])

        hash_value = hashlib.md5(hashstr.encode()).hexdigest()

        if hash_value in hash_dict:
            aggr_message = aggregate_messages(
                message_dict, hash_dict[hash_value], aggr_fields
            )
            hash_dict[hash_value] = aggr_message
        else:
            hash_dict[hash_value] = message_dict

    return hash_dict


def create_message(
    message_type: MessageType, aggr_dict: Dict[str, Dict[str, Union[str, int]]]
) -> str:
    message = Message()

    if message_type == MessageType.summaries:
        message = SummaryMessage()
    elif message_type == MessageType.sync:
        message = SyncMessage()

    header = message.message_header
    message_list = [header]

    for group in aggr_dict.values():
        for k, v in group.items():
            message_list.append(f"{k}: {v}\n")

        message_list.append("%%\n")

    apel_message = "".join(message_list)

    return apel_message


def aggregate_messages(
    new_dict: Dict[str, Union[str, int]],
    aggr_dict: Dict[str, Union[str, int]],
    aggr_fields: List[str],
) -> Dict[str, Union[str, int]]:
    for field in aggr_fields:
        aggr_dict[field] = cast(int, aggr_dict[field]) + cast(int, new_dict[field])

    if "LatestEndTime" in aggr_dict:
        aggr_dict["LatestEndTime"] = new_dict["LatestEndTime"]

    return aggr_dict


def sign_msg(config, msg):
    client_cert = config.messaging.client_cert
    client_key = config.messaging.client_key

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

    ams_message = AmsMessage(data=msg, attributes={"empaid": empaid})

    return ams_message


def send_payload(config, payload):
    host = config.messaging.host
    port = config.messaging.port
    client_cert = config.messaging.client_cert
    client_key = config.messaging.client_key
    project = config.messaging.project
    topic = config.messaging.topic
    retry = config.messaging.retry
    timeout = config.messaging.timeout

    ams = ArgoMessagingService(
        endpoint=host,
        authn_port=port,
        project=project,
        cert=client_cert,
        key=client_key,
    )

    try:
        post = ams.publish(topic, payload, retry=retry, timeout=timeout)
        return post
    except AmsException as e:
        logger.critical(f"Could not send message: {e}")
        raise


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
