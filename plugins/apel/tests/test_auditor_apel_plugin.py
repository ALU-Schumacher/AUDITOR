import pytest
from auditor_apel_plugin.core import (
    get_begin_previous_month,
    create_time_db,
    get_time_db,
    sign_msg,
    get_start_time,
    get_report_time,
    update_time_db,
    create_summary_db,
    get_submit_host,
    get_voms_info,
    replace_record_string,
    get_records,
    get_site_id,
    convert_to_seconds,
    check_sites_in_records,
)
from datetime import datetime, timezone
import sqlite3
import os
import subprocess
import configparser
import pyauditor
from unittest.mock import patch, PropertyMock
import ast
from pathlib import Path, PurePath

test_dir = PurePath(__file__).parent


class FakeAuditorClient:
    def __init__(self, test_case=""):
        self.test_case = test_case

    def get_stopped_since(self, start_time):
        if self.test_case == "pass":
            return "good"
        if self.test_case == "fail_timeout":
            raise RuntimeError("Request timed out")
        if self.test_case == "fail_else":
            raise RuntimeError("Other RuntimeError")


def create_rec_metaless(rec_values, conf):
    rec = pyauditor.Record(rec_values["rec_id"], rec_values["start_time"])
    rec.with_stop_time(rec_values["stop_time"])
    rec.with_component(
        pyauditor.Component(conf["cores_name"], rec_values["n_cores"]).with_score(
            pyauditor.Score(conf["benchmark_name"], rec_values["hepscore"])
        )
    )
    rec.with_component(
        pyauditor.Component(conf["cpu_time_name"], rec_values["tot_cpu"])
    )
    rec.with_component(pyauditor.Component(conf["nnodes_name"], rec_values["n_nodes"]))

    return rec


def create_rec(rec_values, conf):
    rec = pyauditor.Record(rec_values["rec_id"], rec_values["start_time"])
    rec.with_stop_time(rec_values["stop_time"])
    rec.with_component(
        pyauditor.Component(conf["cores_name"], rec_values["n_cores"]).with_score(
            pyauditor.Score(conf["benchmark_name"], rec_values["hepscore"])
        )
    )
    rec.with_component(
        pyauditor.Component(conf["cpu_time_name"], rec_values["tot_cpu"])
    )
    rec.with_component(pyauditor.Component(conf["nnodes_name"], rec_values["n_nodes"]))
    meta = pyauditor.Meta()

    if rec_values["submit_host"] is not None:
        meta.insert(conf["meta_key_submithost"], [rec_values["submit_host"]])
    if rec_values["user_name"] is not None:
        meta.insert(conf["meta_key_username"], [rec_values["user_name"]])
    if rec_values["voms"] is not None:
        meta.insert(conf["meta_key_voms"], [rec_values["voms"]])
    if rec_values["site"] is not None:
        meta.insert(conf["meta_key_site"], [rec_values["site"]])
    rec.with_meta(meta)

    return rec


class TestAuditorApelPlugin:
    def test_get_begin_previous_month(self):
        time_a = datetime(2022, 10, 23, 12, 23, 55)
        time_b = datetime(1970, 1, 1, 00, 00, 00)

        result = get_begin_previous_month(time_a)
        assert result == datetime(2022, 9, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_previous_month(time_b)
        assert result == datetime(1969, 12, 1, 00, 00, 00, tzinfo=timezone.utc)

    def test_create_time_db(self):
        path = ":memory:"
        publish_since_list = [
            "1970-01-01 00:00:00+00:00",
            "2020-01-01 17:23:00+00:00",
            "2022-12-17 20:20:20+01:00",
        ]

        for publish_since in publish_since_list:
            time_db = create_time_db(publish_since, path)
            cur = time_db.cursor()
            cur.execute("SELECT * FROM times")
            result = cur.fetchall()
            cur.close()
            time_db.close()
            time_dt = datetime.strptime(publish_since, "%Y-%m-%d %H:%M:%S%z")
            time_stamp = time_dt.replace(tzinfo=timezone.utc).timestamp()

            assert result == [(time_stamp, datetime(1970, 1, 1, 0, 0, 0))]

    def test_create_time_db_fail(self):
        path = "/home/nonexistent/55/abc/time.db"
        publish_since = "1970-01-01 00:00:00+00:00"

        with pytest.raises(Exception) as pytest_error:
            create_time_db(publish_since, path)
        assert pytest_error.type == sqlite3.OperationalError

        publish_since = "1970-01-01"

        with pytest.raises(Exception) as pytest_error:
            create_time_db(publish_since, path)
        assert pytest_error.type == ValueError

    def test_get_time_db(self):
        path = "/tmp/nonexistent_55_abc_time.db"
        publish_since_list = [
            "1970-01-01 00:00:00+00:00",
            "2020-01-01 17:23:00+00:00",
            "2022-12-17 20:20:20+01:00",
        ]

        conf = configparser.ConfigParser()
        conf["paths"] = {"time_db_path": path}
        conf["site"] = {}

        for publish_since in publish_since_list:
            conf["site"]["publish_since"] = publish_since
            time_db = get_time_db(conf)
            cur = time_db.cursor()
            cur.execute("SELECT * FROM times")
            result = cur.fetchall()
            cur.close()
            time_db.close()
            time_dt = datetime.strptime(publish_since, "%Y-%m-%d %H:%M:%S%z")
            time_stamp = time_dt.replace(tzinfo=timezone.utc).timestamp()
            os.remove(path)

            assert result == [(time_stamp, datetime(1970, 1, 1, 0, 0, 0))]

        for publish_since in publish_since_list:
            conf["site"]["publish_since"] = publish_since
            time_db = create_time_db(publish_since, path)
            time_db.close()
            time_db = get_time_db(conf)
            cur = time_db.cursor()
            cur.execute("SELECT * FROM times")
            result = cur.fetchall()
            cur.close()
            time_db.close()
            time_dt = datetime.strptime(publish_since, "%Y-%m-%d %H:%M:%S%z")
            time_stamp = time_dt.replace(tzinfo=timezone.utc).timestamp()
            os.remove(path)

            assert result == [(time_stamp, datetime(1970, 1, 1, 0, 0, 0))]

    def test_sign_msg(self):
        conf = configparser.ConfigParser()
        conf["authentication"] = {
            "client_cert": Path.joinpath(test_dir, "test_cert.cert"),
            "client_key": Path.joinpath(test_dir, "test_key.key"),
        }

        result = sign_msg(conf, "test")

        with open("/tmp/signed_msg.txt", "wb") as msg_file:
            msg_file.write(result)

        bashCommand = "openssl smime -verify -in /tmp/signed_msg.txt -noverify"
        process = subprocess.Popen(
            bashCommand.split(), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        process.communicate()

        assert process.returncode == 0

    def test_sign_msg_fail(self):
        conf = configparser.ConfigParser()
        conf["authentication"] = {
            "client_cert": "tests/nodir/test_cert.cert",
            "client_key": "tests/no/dir/test_key.key",
        }

        with pytest.raises(Exception) as pytest_error:
            sign_msg(conf, "test")

        assert pytest_error.type == FileNotFoundError

        conf["authentication"]["client_cert"] = str(
            Path.joinpath(test_dir, "test_cert.cert")
        )
        conf["authentication"]["client_key"] = str(
            Path.joinpath(test_dir, "test_key.key")
        )

        result = sign_msg(conf, "test")

        with open("/tmp/signed_msg.txt", "wb") as msg_file:
            msg_file.write(result.replace(b"test", b"TEST"))

        bashCommand = "openssl smime -verify -in /tmp/signed_msg.txt -noverify"
        process = subprocess.Popen(
            bashCommand.split(), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        process.communicate()

        assert process.returncode == 4

    def test_get_start_time(self):
        path = ":memory:"
        publish_since_list = [
            "1970-01-01 00:00:00+00:00",
            "2020-01-01 17:23:00+00:00",
            "2022-12-17 20:20:20+01:00",
        ]

        for publish_since in publish_since_list:
            time_db = create_time_db(publish_since, path)
            result = get_start_time(time_db)
            time_db.close()
            time_dt = datetime.strptime(publish_since, "%Y-%m-%d %H:%M:%S%z")
            time_dt_utc = time_dt.replace(tzinfo=timezone.utc)

            assert result == time_dt_utc

    def test_get_start_time_fail(self):
        path = ":memory:"
        publish_since = "1970-01-01 00:00:00+00:00"

        time_db = create_time_db(publish_since, path)
        drop_column = "ALTER TABLE times DROP last_end_time"

        cur = time_db.cursor()
        cur.execute(drop_column)
        time_db.commit()
        cur.close()
        with pytest.raises(Exception) as pytest_error:
            get_start_time(time_db)
        time_db.close()

        assert pytest_error.type == sqlite3.OperationalError

    def test_get_report_time(self):
        path = ":memory:"
        publish_since = "1970-01-01 00:00:00+00:00"

        time_db = create_time_db(publish_since, path)
        result = get_report_time(time_db)
        time_db.close()

        initial_report_time = datetime(1970, 1, 1, 0, 0, 0)

        assert result == initial_report_time

    def test_get_report_time_fail(self):
        path = ":memory:"
        publish_since = "1970-01-01 00:00:00+00:00"

        time_db = create_time_db(publish_since, path)
        drop_column = "ALTER TABLE times DROP last_report_time"

        cur = time_db.cursor()
        cur.execute(drop_column)
        time_db.commit()
        cur.close()
        with pytest.raises(Exception) as pytest_error:
            get_report_time(time_db)
        time_db.close()

        assert pytest_error.type == sqlite3.OperationalError

    def test_update_time_db(self):
        path = ":memory:"
        publish_since = "1970-01-01 00:00:00+00:00"

        time_db = create_time_db(publish_since, path)
        cur = time_db.cursor()
        cur.row_factory = lambda cursor, row: row[0]

        stop_time_list = [
            datetime(1984, 3, 3, 0, 0, 0),
            datetime(2022, 12, 23, 12, 44, 23),
            datetime(1999, 10, 1, 23, 17, 45),
        ]
        report_time_list = [
            datetime(1993, 4, 4, 0, 0, 0),
            datetime(2100, 8, 19, 14, 16, 11),
            datetime(1887, 2, 27, 0, 11, 31),
        ]

        for stop_time in stop_time_list:
            for report_time in report_time_list:
                update_time_db(time_db, stop_time, report_time)

                cur.execute("SELECT last_end_time FROM times")
                last_end_time_row = cur.fetchall()
                last_end_time = last_end_time_row[0]

                assert last_end_time == stop_time.strftime("%Y-%m-%d %H:%M:%S")

                cur.execute("SELECT last_report_time FROM times")
                last_report_time_row = cur.fetchall()
                last_report_time = last_report_time_row[0]

                assert last_report_time == report_time

                update_time_db(time_db, stop_time.timestamp(), report_time)

                cur.execute("SELECT last_end_time FROM times")
                last_end_time_row = cur.fetchall()
                last_end_time = last_end_time_row[0]

                assert last_end_time == stop_time.timestamp()

        cur.close()
        time_db.close()

    def test_update_time_db_fail(self):
        path = ":memory:"
        publish_since = "1970-01-01 00:00:00+00:00"

        time_db = create_time_db(publish_since, path)
        cur = time_db.cursor()
        cur.row_factory = lambda cursor, row: row[0]

        stop_time = datetime(1984, 3, 3, 0, 0, 0)
        report_time = datetime(2032, 11, 5, 12, 12, 15)

        drop_column = "ALTER TABLE times DROP last_report_time"
        cur.execute(drop_column)
        time_db.commit()

        with pytest.raises(Exception) as pytest_error:
            update_time_db(time_db, stop_time, report_time)

        assert pytest_error.type == sqlite3.OperationalError

        cur.close()
        time_db.close()

    def test_create_summary_db(self):
        site_name_mapping = (
            '{"test-site-1": "TEST_SITE_1", "test-site-2": "TEST_SITE_2"}'
        )
        sites_to_report = '["test-site-1", "test-site-2"]'
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        infrastructure_type = "grid"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        cpu_time_unit = "seconds"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"
        benchmark_type = "hepscore23"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "site_name_mapping": site_name_mapping,
            "sites_to_report": sites_to_report,
            "default_submit_host": default_submit_host,
            "infrastructure_type": infrastructure_type,
            "benchmark_type": benchmark_type,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "cpu_time_unit": cpu_time_unit,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        runtime = 55

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2Fde",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde",
        }

        rec_value_list = [rec_1_values, rec_2_values]
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

            result = create_summary_db(conf, records)

        cur = result.cursor()

        cur.execute("SELECT * FROM records")
        content = cur.fetchall()

        cur.close()
        result.close()

        for idx, rec_values in enumerate(rec_value_list):
            assert (
                content[idx][0]
                == ast.literal_eval(site_name_mapping)[rec_values["site"]]
            )
            assert content[idx][1] == replace_record_string(rec_values["submit_host"])
            assert (
                content[idx][2]
                == replace_record_string(rec_values["voms"]).split("/")[1]
            )
            assert content[idx][3] == replace_record_string(rec_values["voms"])
            assert content[idx][4] is None
            assert content[idx][5] == infrastructure_type
            assert content[idx][6] == rec_values["stop_time"].year
            assert content[idx][7] == rec_values["stop_time"].month
            assert content[idx][8] == rec_values["n_cores"]
            assert content[idx][9] == rec_values["n_nodes"]
            assert content[idx][10] == rec_values["rec_id"]
            assert content[idx][11] == runtime
            assert content[idx][12] == runtime * rec_values["hepscore"]
            assert content[idx][13] == rec_values["tot_cpu"]
            assert content[idx][14] == rec_values["tot_cpu"] * rec_values["hepscore"]
            assert (
                content[idx][15]
                == rec_values["start_time"].replace(tzinfo=timezone.utc).timestamp()
            )
            assert (
                content[idx][16]
                == rec_values["stop_time"].replace(tzinfo=timezone.utc).timestamp()
            )
            assert content[idx][17] == replace_record_string(rec_values["user_name"])

        rec_1_values["user_name"] = None
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

            result = create_summary_db(conf, records)

        cur = result.cursor()

        cur.execute("SELECT * FROM records")
        content = cur.fetchall()

        assert content[0][17] is None

        cur.close()
        result.close()

        rec_2_values["site"] = "test-site-3"
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

            result = create_summary_db(conf, records)

        cur = result.cursor()

        cur.execute("SELECT * FROM records")
        content = cur.fetchall()

        assert len(content) == 1

        cur.close()
        result.close()

    def test_create_summary_db_fail(self):
        site_name_mapping = (
            '{"test-site-1": "TEST_SITE_1", "test-site-2": "TEST_SITE_2"}'
        )
        sites_to_report = '["test-site-1", "test-site-2"]'
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        infrastructure_type = "grid"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        cpu_time_unit = "seconds"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"
        benchmark_type = "hepscore23"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "site_name_mapping": site_name_mapping,
            "sites_to_report": sites_to_report,
            "default_submit_host": default_submit_host,
            "infrastructure_type": infrastructure_type,
            "benchmark_type": benchmark_type,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "cpu_time_unit": cpu_time_unit,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        runtime = 55

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2FRole=production",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas",
        }

        rec_value_list = [rec_1_values, rec_2_values]
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

            conf["auditor"]["cpu_time_name"] = "fail"
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, records)
            assert pytest_error.type == KeyError

            conf["auditor"]["cpu_time_name"] = "TotalCPU"
            conf["auditor"]["nnodes_name"] = "fail"
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, records)
            assert pytest_error.type == KeyError

            conf["auditor"]["nnodes_name"] = "NNodes"
            conf["auditor"]["cores_name"] = "fail"
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, records)
            assert pytest_error.type == KeyError

            conf["auditor"]["cores_name"] = "Cores"
            conf["auditor"]["benchmark_name"] = "fail"
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, records)
            assert pytest_error.type == KeyError

            conf["auditor"]["benchmark_name"] = "hepscore"
            conf["site"]["site_name_mapping"] = '{"test-site-2": "TEST_SITE_2"}'
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, records)
            assert pytest_error.type == KeyError

            rec_metaless = [create_rec_metaless(rec_1_values, conf["auditor"])]
            with pytest.raises(Exception) as pytest_error:
                create_summary_db(conf, rec_metaless)
            assert pytest_error.type == AttributeError

    def test_get_submit_host(self):
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "default_submit_host": default_submit_host,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        runtime = 55

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2FRole=production",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas",
        }

        rec_3_values = {
            "rec_id": "test_record_3",
            "start_time": datetime(2022, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 2,
            "hepscore": 3.0,
            "tot_cpu": 12265325,
            "n_nodes": 1,
            "site": "test-site-2",
            "user": "second_user",
            "submit_host": None,
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas",
        }

        rec_value_list = [rec_1_values, rec_2_values, rec_3_values]
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

        result = get_submit_host(conf, records[0])
        assert result == replace_record_string(rec_1_values["submit_host"])

        result = get_submit_host(conf, records[1])
        assert result == replace_record_string(rec_2_values["submit_host"])

        result = get_submit_host(conf, records[2])
        assert result == default_submit_host

    def test_get_voms_info(self):
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "default_submit_host": default_submit_host,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        runtime = 55

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2FRole=production",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas",
        }

        rec_3_values = {
            "rec_id": "test_record_3",
            "start_time": datetime(2022, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 2,
            "hepscore": 3.0,
            "tot_cpu": 12265325,
            "n_nodes": 1,
            "site": "test-site-2",
            "user": "second_user",
            "submit_host": None,
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde%2FRole=production",
        }

        rec_4_values = {
            "rec_id": "test_record_4",
            "start_time": datetime(2022, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 2,
            "hepscore": 3.0,
            "tot_cpu": 12265325,
            "n_nodes": 1,
            "site": "test-site-2",
            "user": "second_user",
            "submit_host": None,
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde",
        }

        rec_5_values = {
            "rec_id": "test_record_4",
            "start_time": datetime(2022, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 2,
            "hepscore": 3.0,
            "tot_cpu": 12265325,
            "n_nodes": 1,
            "site": "test-site-2",
            "user": "second_user",
            "submit_host": None,
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": None,
        }

        rec_6_values = {
            "rec_id": "test_record_4",
            "start_time": datetime(2022, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 2,
            "hepscore": 3.0,
            "tot_cpu": 12265325,
            "n_nodes": 1,
            "site": "test-site-2",
            "user": "second_user",
            "submit_host": None,
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "atlas",
        }

        rec_value_list = [
            rec_1_values,
            rec_2_values,
            rec_3_values,
            rec_4_values,
            rec_5_values,
            rec_6_values,
        ]
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

        result = get_voms_info(conf, records[0])
        assert result["vo"] == "atlas"
        assert result["vogroup"] == "/atlas"
        assert result["vorole"] == "Role=production"

        result = get_voms_info(conf, records[1])
        assert result["vo"] == "atlas"
        assert result["vogroup"] == "/atlas"
        assert result["vorole"] is None

        result = get_voms_info(conf, records[2])
        assert result["vo"] == "atlas"
        assert result["vogroup"] == "/atlas/de"
        assert result["vorole"] == "Role=production"

        result = get_voms_info(conf, records[3])
        assert result["vo"] == "atlas"
        assert result["vogroup"] == "/atlas/de"
        assert result["vorole"] is None

        result = get_voms_info(conf, records[4])
        assert result["vo"] is None
        assert result["vogroup"] is None
        assert result["vorole"] is None

        result = get_voms_info(conf, records[5])
        assert result["vo"] is None
        assert result["vogroup"] is None
        assert result["vorole"] is None

    def test_replace_record_string(self):
        test_str_1 = "abcd"
        test_str_2 = "%2Fa%2Fb%2Fc%2Fd%2F"

        result = replace_record_string(test_str_1)
        assert result == "abcd"

        result = replace_record_string(test_str_2)
        assert result == "/a/b/c/d/"

    def test_get_records(self):
        client = FakeAuditorClient("pass")

        result = get_records(client, 42, 1)
        assert result == "good"

    def test_get_records_fail(self):
        client = FakeAuditorClient("fail_timeout")

        with pytest.raises(SystemExit) as pytest_error:
            get_records(client, 42, 1)
        assert pytest_error.type == SystemExit

        client = FakeAuditorClient("fail_else")

        with pytest.raises(Exception) as pytest_error:
            get_records(client, 42, 1)
        assert pytest_error.type == RuntimeError

    def test_get_site_id(self):
        site_name_mapping = (
            '{"test-site-1": "TEST_SITE_1", "test-site-2": "TEST_SITE_2"}'
        )
        sites_to_report = '["test-site-1", "test-site-2"]'
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        infrastructure_type = "grid"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "site_name_mapping": site_name_mapping,
            "sites_to_report": sites_to_report,
            "default_submit_host": default_submit_host,
            "infrastructure_type": infrastructure_type,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2Fde",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde",
        }

        rec_1 = create_rec(rec_1_values, conf["auditor"])
        rec_2 = create_rec(rec_2_values, conf["auditor"])

        result = get_site_id(conf, rec_1)
        assert result == rec_1_values["site"]

        result = get_site_id(conf, rec_2)
        assert result == rec_2_values["site"]

    def test_get_site_id_fail(self):
        site_name_mapping = (
            '{"test-site-1": "TEST_SITE_1", "test-site-2": "TEST_SITE_2"}'
        )
        sites_to_report = '["test-site-1", "test-site-2"]'
        default_submit_host = "https://default.submit_host.de:1234/xxx"
        infrastructure_type = "grid"
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "site_name_mapping": site_name_mapping,
            "sites_to_report": sites_to_report,
            "default_submit_host": default_submit_host,
            "infrastructure_type": infrastructure_type,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": None,
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2Fde",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde",
        }

        rec_1 = create_rec(rec_1_values, conf["auditor"])
        rec_2 = create_rec_metaless(rec_2_values, conf["auditor"])

        with pytest.raises(Exception) as pytest_error:
            get_site_id(conf, rec_1)
        assert pytest_error.type == TypeError

        with pytest.raises(Exception) as pytest_error:
            get_site_id(conf, rec_2)
        assert pytest_error.type == AttributeError

    def test_convert_to_seconds(self):
        cpu_time_name = "TotalCPU"
        cpu_time_unit = "seconds"

        conf = configparser.ConfigParser()
        conf["auditor"] = {
            "cpu_time_name": cpu_time_name,
            "cpu_time_unit": cpu_time_unit,
        }

        result = convert_to_seconds(conf, 1100)
        assert result == 1100

        result = convert_to_seconds(conf, 1500)
        assert result == 1500

        conf["auditor"]["cpu_time_unit"] = "milliseconds"

        result = convert_to_seconds(conf, 1100)
        assert result == 1

        result = convert_to_seconds(conf, 1500)
        assert result == 2

    def test_convert_to_seconds_fail(self):
        cpu_time_name = "TotalCPU"
        cpu_time_unit = "hours"

        conf = configparser.ConfigParser()
        conf["auditor"] = {
            "cpu_time_name": cpu_time_name,
            "cpu_time_unit": cpu_time_unit,
        }

        with pytest.raises(Exception) as pytest_error:
            convert_to_seconds(conf, 1100)
        assert pytest_error.type == ValueError

    def test_check_sites_in_records(self):
        sites_to_report = '["test-site-1", "test-site-2"]'
        benchmark_name = "hepscore"
        cores_name = "Cores"
        cpu_time_name = "TotalCPU"
        nnodes_name = "NNodes"
        meta_key_site = "site_id"
        meta_key_submithost = "headnode"
        meta_key_voms = "voms"
        meta_key_username = "subject"

        conf = configparser.ConfigParser()
        conf["site"] = {
            "sites_to_report": sites_to_report,
        }
        conf["auditor"] = {
            "benchmark_name": benchmark_name,
            "cores_name": cores_name,
            "cpu_time_name": cpu_time_name,
            "nnodes_name": nnodes_name,
            "meta_key_site": meta_key_site,
            "meta_key_submithost": meta_key_submithost,
            "meta_key_voms": meta_key_voms,
            "meta_key_username": meta_key_username,
        }

        runtime = 55

        rec_1_values = {
            "rec_id": "test_record_1",
            "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
            "n_cores": 8,
            "hepscore": 10.0,
            "tot_cpu": 15520000,
            "n_nodes": 1,
            "site": "test-site-1",
            "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
            "voms": "%2Fatlas%2Fde",
        }

        rec_2_values = {
            "rec_id": "test_record_2",
            "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
            "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
            "n_cores": 1,
            "hepscore": 23.0,
            "tot_cpu": 12234325,
            "n_nodes": 2,
            "site": "test-site-2",
            "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
            "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
            "voms": "%2Fatlas%2Fde",
        }

        rec_value_list = [rec_1_values, rec_2_values]
        records = []

        with patch(
            "pyauditor.Record.runtime", new_callable=PropertyMock
        ) as mocked_runtime:
            mocked_runtime.return_value = runtime

            for r_values in rec_value_list:
                rec = create_rec(r_values, conf["auditor"])
                records.append(rec)

            result = check_sites_in_records(conf, records)
            assert len(result) == 2

            conf["site"]["sites_to_report"] = '["test-site-1"]'

            result = check_sites_in_records(conf, records)
            assert len(result) == 1

            conf["site"]["sites_to_report"] = '["test-site-3"]'

            result = check_sites_in_records(conf, records)
            assert len(result) == 0
