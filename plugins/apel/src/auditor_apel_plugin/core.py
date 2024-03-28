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
from pyauditor import Value, Operator, MetaOperator, MetaQuery, QueryBuilder


def get_records(config, client, start_time, delay_time, site=None, end_time=None):
    sites_to_report = config["site"]["sites_to_report"]
    meta_key_site = config["auditor"]["meta_key_site"]

    site_ids = []

    if site is not None:
        site_ids = sites_to_report[site]
        logging.info(f"Getting records for site {site} with site_ids: {site_ids}")
    else:
        for k, v in sites_to_report.items():
            site_ids.extend(v)

        logging.info(
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
                logging.warning(
                    f"Call to AUDITOR timed out {timeout_counter}/3! "
                    f"Trying again in {timeout_counter * delay_time}s"
                )
                sleep(timeout_counter * delay_time)
            else:
                logging.critical(e)
                raise

    logging.critical(
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
    time_json_path = config["time_json_path"]

    try:
        with open(time_json_path, "r", encoding="utf-8") as f:
            time_dict = json.load(f)
    except FileNotFoundError:
        logging.warning(f"Path {time_json_path} not found, creating new time json")
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
        logging.critical(f"Path {time_json_path} not found, could not create time json")
        raise

    return time_dict


def get_start_time(config, time_dict, site):
    try:
        start_time = datetime.fromisoformat(time_dict["site_end_times"][site])
    except KeyError:
        start_time = config["site"]["publish_since"]

    return start_time


def get_report_time(time_dict):
    report_time = datetime.fromisoformat(time_dict["last_report_time"])

    return report_time


def update_time_json(config, time_dict, site, stop_time, report_time):
    time_json_path = config["time_json_path"]

    time_dict["last_report_time"] = report_time.isoformat()
    time_dict["site_end_times"][site] = stop_time.isoformat()

    try:
        with open(time_json_path, "w", encoding="utf-8") as f:
            json.dump(time_dict, f)
    except FileNotFoundError:
        logging.critical(f"Path {time_json_path} not found, could not update time json")
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
        logging.critical(f"No meta data found in {record.record_id}, aborting")
        raise
    except TypeError:
        logging.critical(f"No site name found in {record.record_id}, aborting")
        raise


def get_submit_host(config, record):
    meta_key_submithost = config["auditor"]["meta_key_submithost"]
    default_submit_host = config["site"]["default_submit_host"]

    try:
        submit_host = replace_record_string(record.meta.get(meta_key_submithost)[0])
    except TypeError:
        logging.warning(
            f"No {meta_key_submithost} found in record {record.record_id}, "
            f"sending default SubmitHost {default_submit_host}"
        )
        submit_host = default_submit_host

    return submit_host


def get_voms_info(config, record):
    meta_key_voms = config["auditor"]["meta_key_voms"]
    voms_dict = {}

    try:
        voms_string = replace_record_string(record.meta.get(meta_key_voms)[0])
    except TypeError:
        logging.warning(
            f"No VOMS information found in {record.record_id}, "
            "not sending VO, VOGroup, and VORole"
        )

        voms_dict["vo"] = None
        voms_dict["vogroup"] = None
        voms_dict["vorole"] = None

        return voms_dict

    if not voms_string.startswith("/"):
        logging.warning(
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
        logging.warning(
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


def create_summary_db(config, records):
    create_table_sql = """
                       CREATE TABLE IF NOT EXISTS records(
                           site TEXT NOT NULL,
                           submithost TEXT NOT NULL,
                           vo TEXT,
                           vogroup TEXT,
                           vorole TEXT,
                           infrastructure TEXT NOT NULL,
                           year INTEGER NOT NULL,
                           month INTEGER NOT NULL,
                           cpucount INTEGER NOT NULL,
                           nodecount INTEGER NOT NULL,
                           recordid TEXT UNIQUE NOT NULL,
                           runtime INTEGER NOT NULL,
                           normruntime INTEGER NOT NULL,
                           cputime INTEGER NOT NULL,
                           normcputime INTEGER NOT NULL,
                           starttime INTEGER NOT NULL,
                           stoptime INTEGER NOT NULL,
                           user TEXT,
                           benchmarktype TEXT NOT NULL,
                           benchmarkvalue FLOAT NOT NULL
                       )
                       """

    insert_record_sql = """
                        INSERT INTO records(
                            site,
                            submithost,
                            vo,
                            vogroup,
                            vorole,
                            infrastructure,
                            year,
                            month,
                            cpucount,
                            nodecount,
                            recordid,
                            runtime,
                            normruntime,
                            cputime,
                            normcputime,
                            starttime,
                            stoptime,
                            user,
                            benchmarktype,
                            benchmarkvalue
                        )
                        VALUES(
                            ?, ?, ?, ?,
                            ?, ?, ?, ?,
                            ?, ?, ?, ?,
                            ?, ?, ?, ?,
                            ?, ?, ?, ?
                        )
                        """

    try:
        conn = sqlite3.connect(":memory:")
        cur = conn.cursor()
        cur.execute(create_table_sql)
    except Error as e:
        logging.critical(e)
        raise

    sites_to_report = config["site"]["sites_to_report"]
    infrastructure = config["site"]["infrastructure_type"]
    benchmark_type = config["site"]["benchmark_type"]
    benchmark_name = config["auditor"]["benchmark_name"]
    cores_name = config["auditor"]["cores_name"]
    cpu_time_name = config["auditor"]["cpu_time_name"]
    nnodes_name = config["auditor"]["nnodes_name"]
    meta_key_username = config["auditor"]["meta_key_username"]

    for r in records:
        site_id = get_site_id(config, r)

        for k, v in sites_to_report.items():
            if site_id in v:
                site_name = k
                break

        submit_host = get_submit_host(config, r)

        voms_dict = get_voms_info(config, r)

        try:
            user_name = replace_record_string(r.meta.get(meta_key_username)[0])
        except TypeError:
            logging.warning(
                f"No GlobalUserName found in {r.record_id}, not sending GlobalUserName"
            )
            user_name = None

        year = r.stop_time.replace(tzinfo=timezone.utc).year
        month = r.stop_time.replace(tzinfo=timezone.utc).month

        component_dict = {}
        score_dict = {}

        for c in r.components:
            component_dict[c.name] = c

        try:
            cputime = component_dict[cpu_time_name].amount
        except KeyError:
            logging.critical(f"no {cpu_time_name} in components")
            raise

        cputime = convert_to_seconds(config, cputime)

        try:
            nodecount = component_dict[nnodes_name].amount
        except KeyError:
            logging.critical(f"no {nnodes_name} in components")
            raise

        try:
            cpucount = component_dict[cores_name].amount
            for s in component_dict[cores_name].scores:
                score_dict[s.name] = s.value
        except KeyError:
            logging.critical(f"no {cores_name} in components")
            raise

        try:
            benchmark_value = score_dict[benchmark_name]
        except KeyError:
            logging.critical(f"no {benchmark_name} in scores")
            raise

        norm_runtime = r.runtime * benchmark_value
        norm_cputime = cputime * benchmark_value

        data_tuple = (
            site_name,
            submit_host,
            voms_dict["vo"],
            voms_dict["vogroup"],
            voms_dict["vorole"],
            infrastructure,
            year,
            month,
            cpucount,
            nodecount,
            r.record_id,
            r.runtime,
            norm_runtime,
            cputime,
            norm_cputime,
            r.start_time.replace(tzinfo=timezone.utc).timestamp(),
            r.stop_time.replace(tzinfo=timezone.utc).timestamp(),
            user_name,
            benchmark_type,
            benchmark_value,
        )
        try:
            cur.execute(insert_record_sql, data_tuple)
        except Error as e:
            logging.critical(e)
            raise

    try:
        conn.commit()
        cur.close()
    except Error as e:
        logging.critical(e)
        raise

    return conn


def create_sync_db(config, records):
    create_table_sql = """
                       CREATE TABLE IF NOT EXISTS records(
                           site TEXT NOT NULL,
                           submithost TEXT NOT NULL,
                           year INTEGER NOT NULL,
                           month INTEGER NOT NULL,
                           recordid TEXT UNIQUE NOT NULL
                       )
                       """

    insert_record_sql = """
                        INSERT INTO records(
                            site,
                            submithost,
                            year,
                            month,
                            recordid
                        )
                        VALUES(
                            ?, ?, ?, ?, ?
                        )
                        """

    try:
        conn = sqlite3.connect(":memory:")
        cur = conn.cursor()
        cur.execute(create_table_sql)
    except Error as e:
        logging.critical(e)
        raise

    sites_to_report = config["site"]["sites_to_report"]

    for r in records:
        site_id = get_site_id(config, r)

        for k, v in sites_to_report.items():
            if site_id in v:
                site_name = k
                break

        submit_host = get_submit_host(config, r)

        year = r.stop_time.replace(tzinfo=timezone.utc).year
        month = r.stop_time.replace(tzinfo=timezone.utc).month

        data_tuple = (
            site_name,
            submit_host,
            year,
            month,
            r.record_id,
        )
        try:
            cur.execute(insert_record_sql, data_tuple)
        except Error as e:
            logging.critical(e)
            raise

    try:
        conn.commit()
        cur.close()
    except Error as e:
        logging.critical(e)
        raise

    return conn


def group_summary_db(summary_db):
    group_sql = """
                 SELECT site,
                        submithost,
                        vo,
                        vogroup,
                        vorole,
                        infrastructure,
                        year,
                        month,
                        cpucount,
                        nodecount,
                        COUNT(recordid) as jobcount,
                        SUM(runtime) as runtime,
                        SUM(normruntime) as norm_runtime,
                        SUM(cputime) as cputime,
                        SUM(normcputime) as norm_cputime,
                        MIN(stoptime) as min_stoptime,
                        MAX(stoptime) as max_stoptime,
                        user,
                        benchmarktype,
                        benchmarkvalue
                 FROM records
                 GROUP BY site,
                          submithost,
                          vo,
                          vogroup,
                          vorole,
                          infrastructure,
                          year,
                          month,
                          cpucount,
                          nodecount,
                          user,
                          benchmarktype,
                          benchmarkvalue
                """

    summary_db.row_factory = sqlite3.Row
    cur = summary_db.cursor()
    cur.execute(group_sql)
    grouped_summary_list = cur.fetchall()
    cur.close()
    summary_db.close()

    return grouped_summary_list


def group_sync_db(sync_db):
    sync_db.row_factory = sqlite3.Row
    cur = sync_db.cursor()
    group_sql = """
                SELECT site,
                       submithost,
                       year,
                       month,
                       COUNT(recordid) as jobcount
                FROM records
                GROUP BY site,
                         submithost,
                         year,
                         month
                """

    cur.execute(group_sql)
    grouped_sync_list = cur.fetchall()
    cur.close()
    sync_db.close()

    return grouped_sync_list


def create_summary(config, grouped_summary_list):
    apel_style = config["site"].get("apel_style", "Test")

    if apel_style == "APEL-v0.2":
        summary = "APEL-summary-job-message: v0.2\n"
    elif apel_style == "APEL-v0.3":
        summary = "APEL-summary-job-message: v0.3\n"
    elif apel_style == "ARC":
        summary = "APEL-summary-job-message: v0.2\n"
    elif apel_style == "Test":
        summary = "APEL-summary-job-message: v0.3\n"
    else:
        logging.critical(
            f"No such style: {apel_style}, please fix apel_style in the config"
        )
        raise ValueError

    for entry in grouped_summary_list:
        summary += f"Site: {entry['site']}\n"
        summary += f"Month: {entry['month']}\n"
        summary += f"Year: {entry['year']}\n"
        if entry["user"] is not None:
            summary += f"GlobalUserName: {entry['user']}\n"
        if entry["vo"] is not None:
            if apel_style == "APEL-v0.2":
                summary += f"Group: {entry['vo']}\n"
            else:
                summary += f"VO: {entry['vo']}\n"
        if entry["vogroup"] is not None:
            summary += f"VOGroup: {entry['vogroup']}\n"
        if entry["vorole"] is not None:
            summary += f"VORole: {entry['vorole']}\n"
        if apel_style != "APEL-v0.2":
            summary += f"SubmitHost: {entry['submithost']}\n"
            summary += f"InfrastructureType: {entry['infrastructure']}\n"
            summary += f"Processors: {entry['cpucount']}\n"
            summary += f"NodeCount: {entry['nodecount']}\n"
        summary += f"EarliestEndTime: {entry['min_stoptime']}\n"
        summary += f"LatestEndTime: {entry['max_stoptime']}\n"
        summary += f"WallDuration : {int(entry['runtime'])}\n"
        summary += f"CpuDuration: {int(entry['cputime'])}\n"
        if apel_style != "ARC":
            summary += f"NormalisedWallDuration: {int(entry['norm_runtime'])}\n"
            summary += f"NormalisedCpuDuration: {int(entry['norm_cputime'])}\n"
        if apel_style in ["ARC", "Test"]:
            summary += f"ServiceLevelType: {entry['benchmarktype']}\n"
            summary += f"ServiceLevel: {entry['benchmarkvalue']}\n"
        summary += f"NumberOfJobs: {entry['jobcount']}\n"
        summary += "%%\n"

    return summary


def create_sync(sync_db):
    sync = "APEL-sync-message: v0.1\n"

    for entry in sync_db:
        sync += f"Site: {entry['site']}\n"
        sync += f"Month: {entry['month']}\n"
        sync += f"Year: {entry['year']}\n"
        sync += f"SubmitHost: {entry['submithost']}\n"
        sync += f"NumberOfJobs: {entry['jobcount']}\n"
        sync += "%%\n"

    return sync


def get_token(config):
    auth_url = config["authentication"]["auth_url"]
    client_cert = config["authentication"]["client_cert"]
    client_key = config["authentication"]["client_key"]
    verify_ca = config["authentication"]["verify_ca"]
    if verify_ca:
        ca_path = config["authentication"]["ca_path"]
    else:
        ca_path = False

    response = requests.get(auth_url, cert=(client_cert, client_key), verify=ca_path)
    token = response.json()["token"]

    return token


def sign_msg(config, msg):
    client_cert = config["authentication"]["client_cert"]
    client_key = config["authentication"]["client_key"]

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
    ams_url = config["authentication"]["ams_url"]
    verify_ca = config["authentication"]["verify_ca"]

    if verify_ca:
        ca_path = config["authentication"]["ca_path"]
    else:
        ca_path = False

    logging.debug(f"{ams_url}{token}")
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
        logging.critical(
            f"Unknown unit for {cpu_time_name}: {cpu_time_unit}. "
            "Possible values are seconds or milliseconds."
        )
        raise ValueError


def check_sites_in_records(config, records):
    sites_to_report = config["site"]["sites_to_report"]

    logging.debug(f"Sites to report from config: {list(sites_to_report.keys())}")

    sites_in_records = {get_site_id(config, r) for r in records}
    sites = []

    for site_id in sites_in_records:
        for k, v in sites_to_report.items():
            if site_id in v:
                sites.append(k)
                break

    logging.debug(f"Sites found in records: {sites}")

    return sites
