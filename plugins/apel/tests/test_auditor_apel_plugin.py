import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path, PurePath

import pyauditor
import pytest
import yaml

from auditor_apel_plugin.config import Config, get_loaders
from auditor_apel_plugin.core import (
    convert_to_seconds,
    create_time_json,
    get_begin_current_month,
    get_begin_previous_month,
    get_records,
    get_report_time,
    get_time_json,
    sign_msg,
    update_time_json,
)

test_dir = PurePath(__file__).parent


class FakeAuditorClient:
    def __init__(self, test_case=""):
        self.test_case = test_case

    def advanced_query(self, start_time):
        if self.test_case == "pass":
            return "good"
        if self.test_case == "fail":
            raise RuntimeError("RuntimeError")


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

    def test_get_begin_current_month(self):
        time_a = datetime(2022, 10, 23, 12, 23, 55)
        time_b = datetime(1970, 1, 1, 00, 00, 00)

        result = get_begin_current_month(time_a)
        assert result == datetime(2022, 10, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_current_month(time_b)
        assert result == datetime(1970, 1, 1, 00, 00, 00, tzinfo=timezone.utc)

    def test_create_time_json(self):
        path = "/tmp/test.json"

        result = create_time_json(path)

        assert result["site_end_times"] == {}
        assert result["last_report_time"] == "1970-01-01T00:00:00"

        with open("/tmp/test.json", "r", encoding="utf-8") as f:
            result = json.load(f)

        os.remove(path)

        assert result["site_end_times"] == {}
        assert result["last_report_time"] == "1970-01-01T00:00:00"

    def test_create_time_json_fail(self):
        path = "/home/nonexistent/55/abc/time.db"

        with pytest.raises(Exception) as pytest_error:
            create_time_json(path)

        assert pytest_error.type is FileNotFoundError

    def test_get_time_json(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/nonexistent_55_abc_time.db"

        config.plugin.time_json_path = path

        result = get_time_json(config)
        os.remove(path)

        assert result["site_end_times"] == {}
        assert result["last_report_time"] == "1970-01-01T00:00:00"

        create_time_json(path)
        result = get_time_json(config)
        os.remove(path)

        assert result["site_end_times"] == {}
        assert result["last_report_time"] == "1970-01-01T00:00:00"

    def test_sign_msg(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        config.messaging.client_cert = Path.joinpath(test_dir, "test_cert.cert")
        config.messaging.client_key = Path.joinpath(test_dir, "test_key.key")

        result = sign_msg(config, "test")

        with open("/tmp/signed_msg.txt", "wb") as msg_file:
            msg_file.write(result)

        bashCommand = "openssl smime -verify -in /tmp/signed_msg.txt -noverify"
        process = subprocess.Popen(
            bashCommand.split(), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        process.communicate()

        assert process.returncode == 0

    def test_sign_msg_fail(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        config.messaging.client_cert = "tests/nodir/test_cert.cert"
        config.messaging.client_key = "tests/no/dir/test_key.key"

        with pytest.raises(Exception) as pytest_error:
            sign_msg(config, "test")

        assert pytest_error.type is FileNotFoundError

        config.messaging.client_cert = str(Path.joinpath(test_dir, "test_cert.cert"))
        config.messaging.client_key = str(Path.joinpath(test_dir, "test_key.key"))

        result = sign_msg(config, "test")

        with open("/tmp/signed_msg.txt", "wb") as msg_file:
            msg_file.write(result.replace(b"test", b"TEST"))

        bashCommand = "openssl smime -verify -in /tmp/signed_msg.txt -noverify"
        process = subprocess.Popen(
            bashCommand.split(), stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        process.communicate()

        assert process.returncode == 4

    def test_get_report_time(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"
        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

        result = get_report_time(time_dict)

        initial_report_time = datetime(1970, 1, 1, 0, 0, 0)

        assert result == initial_report_time

        new_report_time = datetime(2023, 2, 10, 11, 13, 45)
        stop_time = datetime(2002, 12, 3, 0, 45, 0, tzinfo=timezone.utc)

        update_time_json(
            config,
            time_dict,
            "TEST-2",
            stop_time,
            new_report_time,
        )

        result = get_report_time(time_dict)
        os.remove(path)
        assert result == new_report_time

    def test_update_time_json(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"

        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

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
        site_list = ["SITE_A", "SITE_B"]

        for stop_time in stop_time_list:
            for report_time in report_time_list:
                for site in site_list:
                    update_time_json(config, time_dict, site, stop_time, report_time)

                    with open(path, "r", encoding="utf-8") as f:
                        time_dict_test = json.load(f)
                        last_end_time = time_dict_test["site_end_times"][site]
                        last_report_time = time_dict_test["last_report_time"]

                    assert last_end_time == stop_time.isoformat()
                    assert last_report_time == report_time.isoformat()

        os.remove(path)

    def test_update_time_json_fail(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"

        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

        stop_time = datetime(1984, 3, 3, 0, 0, 0)
        report_time = datetime(1993, 4, 4, 0, 0, 0)
        site = "SITE_A"

        os.remove(path)

        path_new = "/dfghdfh/test.json"
        config.plugin.time_json_path = path_new

        with pytest.raises(Exception) as pytest_error:
            update_time_json(config, time_dict, site, stop_time, report_time)

        assert pytest_error.type is FileNotFoundError

    # def test_create_summary_db(self):
    #     sites_to_report = {
    #         "TEST_SITE_1": ["test-site-1"],
    #         "TEST_SITE_2": ["test-site-2"],
    #     }
    #     default_submit_host = "https://default.submit_host.de:1234/xxx"
    #     infrastructure_type = "grid"
    #     benchmark_name = "hepscore"
    #     cores_name = "Cores"
    #     cpu_time_name = "TotalCPU"
    #     cpu_time_unit = "seconds"
    #     nnodes_name = "NNodes"
    #     meta_key_site = "site_id"
    #     meta_key_submithost = "headnode"
    #     meta_key_voms = "voms"
    #     meta_key_username = "subject"
    #     benchmark_type = "hepscore23"

    #     conf = {}
    #     conf["site"] = {
    #         "sites_to_report": sites_to_report,
    #         "default_submit_host": default_submit_host,
    #         "infrastructure_type": infrastructure_type,
    #         "benchmark_type": benchmark_type,
    #     }
    #     conf["auditor"] = {
    #         "benchmark_name": benchmark_name,
    #         "cores_name": cores_name,
    #         "cpu_time_name": cpu_time_name,
    #         "cpu_time_unit": cpu_time_unit,
    #         "nnodes_name": nnodes_name,
    #         "meta_key_site": meta_key_site,
    #         "meta_key_submithost": meta_key_submithost,
    #         "meta_key_voms": meta_key_voms,
    #         "meta_key_username": meta_key_username,
    #     }

    #     runtime = 55

    #     rec_1_values = {
    #         "rec_id": "test_record_1",
    #         "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
    #         "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
    #         "n_cores": 8,
    #         "hepscore": 10.0,
    #         "tot_cpu": 15520000,
    #         "n_nodes": 1,
    #         "site": "test-site-1",
    #         "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
    #         "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
    #         "voms": "%2Fatlas%2Fde",
    #     }

    #     rec_2_values = {
    #         "rec_id": "test_record_2",
    #         "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
    #         "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
    #         "n_cores": 1,
    #         "hepscore": 23.0,
    #         "tot_cpu": 12234325,
    #         "n_nodes": 2,
    #         "site": "test-site-2",
    #         "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
    #         "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
    #         "voms": "%2Fatlas%2Fde",
    #     }

    #     rec_value_list = [rec_1_values, rec_2_values]
    #     records = []

    #     with patch(
    #         "pyauditor.Record.runtime", new_callable=PropertyMock
    #     ) as mocked_runtime:
    #         mocked_runtime.return_value = runtime

    #         for r_values in rec_value_list:
    #             rec = create_rec(r_values, conf["auditor"])
    #             records.append(rec)

    #         result = create_summary_db(conf, records)

    #     cur = result.cursor()

    #     cur.execute("SELECT * FROM records")
    #     content = cur.fetchall()

    #     cur.close()
    #     result.close()

    #     for idx, rec_values in enumerate(rec_value_list):
    #         assert content[idx][0] == list(sites_to_report.keys())[idx]
    #         assert content[idx][1] == replace_record_string(rec_values["submit_host"])
    #         assert (
    #             content[idx][2]
    #             == replace_record_string(rec_values["voms"]).split("/")[1]
    #         )
    #         assert content[idx][3] == replace_record_string(rec_values["voms"])
    #         assert content[idx][4] is None
    #         assert content[idx][5] == infrastructure_type
    #         assert content[idx][6] == rec_values["stop_time"].year
    #         assert content[idx][7] == rec_values["stop_time"].month
    #         assert content[idx][8] == rec_values["n_cores"]
    #         assert content[idx][9] == rec_values["n_nodes"]
    #         assert content[idx][10] == rec_values["rec_id"]
    #         assert content[idx][11] == runtime
    #         assert content[idx][12] == runtime * rec_values["hepscore"]
    #         assert content[idx][13] == rec_values["tot_cpu"]
    #         assert content[idx][14] == rec_values["tot_cpu"] * rec_values["hepscore"]
    #         assert (
    #             content[idx][15]
    #             == rec_values["start_time"].replace(tzinfo=timezone.utc).timestamp()
    #         )
    #         assert (
    #             content[idx][16]
    #             == rec_values["stop_time"].replace(tzinfo=timezone.utc).timestamp()
    #         )
    #         assert content[idx][17] == replace_record_string(rec_values["user_name"])

    #     rec_1_values["user_name"] = None
    #     records = []

    #     with patch(
    #         "pyauditor.Record.runtime", new_callable=PropertyMock
    #     ) as mocked_runtime:
    #         mocked_runtime.return_value = runtime

    #         for r_values in rec_value_list:
    #             rec = create_rec(r_values, conf["auditor"])
    #             records.append(rec)

    #         result = create_summary_db(conf, records)

    #     cur = result.cursor()

    #     cur.execute("SELECT * FROM records")
    #     content = cur.fetchall()

    #     assert content[0][17] is None

    #     cur.close()
    #     result.close()

    # def test_create_summary_db_fail(self):
    #     sites_to_report = {
    #         "TEST_SITE_1": ["test-site-1"],
    #         "TEST_SITE_2": ["test-site-2"],
    #     }
    #     default_submit_host = "https://default.submit_host.de:1234/xxx"
    #     infrastructure_type = "grid"
    #     benchmark_name = "hepscore"
    #     cores_name = "Cores"
    #     cpu_time_name = "TotalCPU"
    #     cpu_time_unit = "seconds"
    #     nnodes_name = "NNodes"
    #     meta_key_site = "site_id"
    #     meta_key_submithost = "headnode"
    #     meta_key_voms = "voms"
    #     meta_key_username = "subject"
    #     benchmark_type = "hepscore23"

    #     conf = {}
    #     conf["site"] = {
    #         "sites_to_report": sites_to_report,
    #         "default_submit_host": default_submit_host,
    #         "infrastructure_type": infrastructure_type,
    #         "benchmark_type": benchmark_type,
    #     }
    #     conf["auditor"] = {
    #         "benchmark_name": benchmark_name,
    #         "cores_name": cores_name,
    #         "cpu_time_name": cpu_time_name,
    #         "cpu_time_unit": cpu_time_unit,
    #         "nnodes_name": nnodes_name,
    #         "meta_key_site": meta_key_site,
    #         "meta_key_submithost": meta_key_submithost,
    #         "meta_key_voms": meta_key_voms,
    #         "meta_key_username": meta_key_username,
    #     }

    #     runtime = 55

    #     rec_1_values = {
    #         "rec_id": "test_record_1",
    #         "start_time": datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
    #         "stop_time": datetime(1985, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
    #         "n_cores": 8,
    #         "hepscore": 10.0,
    #         "tot_cpu": 15520000,
    #         "n_nodes": 1,
    #         "site": "test-site-1",
    #         "submit_host": "https:%2F%2Ftest1.submit_host.de:1234%2Fxxx",
    #         "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test1: test1",
    #         "voms": "%2Fatlas%2FRole=production",
    #     }

    #     rec_2_values = {
    #         "rec_id": "test_record_2",
    #         "start_time": datetime(2023, 1, 1, 14, 24, 11).astimezone(tz=timezone.utc),
    #         "stop_time": datetime(2023, 1, 2, 7, 11, 45).astimezone(tz=timezone.utc),
    #         "n_cores": 1,
    #         "hepscore": 23.0,
    #         "tot_cpu": 12234325,
    #         "n_nodes": 2,
    #         "site": "test-site-2",
    #         "submit_host": "https:%2F%2Ftest2.submit_host.de:1234%2Fxxx",
    #         "user_name": "%2FDC=ch%2FDC=cern%2FOU=Users%2FCN=test2: test2",
    #         "voms": "%2Fatlas",
    #     }

    #     rec_value_list = [rec_1_values, rec_2_values]
    #     records = []

    #     with patch(
    #         "pyauditor.Record.runtime", new_callable=PropertyMock
    #     ) as mocked_runtime:
    #         mocked_runtime.return_value = runtime

    #         for r_values in rec_value_list:
    #             rec = create_rec(r_values, conf["auditor"])
    #             records.append(rec)

    #         conf["auditor"]["cpu_time_name"] = "fail"
    #         with pytest.raises(Exception) as pytest_error:
    #             create_summary_db(conf, records)
    #         assert pytest_error.type is KeyError

    #         conf["auditor"]["cpu_time_name"] = "TotalCPU"
    #         conf["auditor"]["nnodes_name"] = "fail"
    #         with pytest.raises(Exception) as pytest_error:
    #             create_summary_db(conf, records)
    #         assert pytest_error.type is KeyError

    #         conf["auditor"]["nnodes_name"] = "NNodes"
    #         conf["auditor"]["cores_name"] = "fail"
    #         with pytest.raises(Exception) as pytest_error:
    #             create_summary_db(conf, records)
    #         assert pytest_error.type is KeyError

    #         conf["auditor"]["cores_name"] = "Cores"
    #         conf["auditor"]["benchmark_name"] = "fail"
    #         with pytest.raises(Exception) as pytest_error:
    #             create_summary_db(conf, records)
    #         assert pytest_error.type is KeyError

    #         rec_metaless = [create_rec_metaless(rec_1_values, conf["auditor"])]
    #         with pytest.raises(Exception) as pytest_error:
    #             create_summary_db(conf, rec_metaless)
    #         assert pytest_error.type is AttributeError

    def test_get_records(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        client = FakeAuditorClient("pass")

        sites_to_report = {"TEST_SITE_1": ["test-site-1"]}
        meta_key_site = ["site_id"]

        config.site.sites_to_report = sites_to_report
        config.auditor.site_meta_field = meta_key_site

        start_time_str = "2022-12-17 20:20:20+01:00"
        start_time = datetime.fromisoformat(start_time_str).replace(tzinfo=timezone.utc)

        result = get_records(config, client, start_time)
        assert "".join(result) == "good"

    def test_get_records_fail(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        sites_to_report = {"TEST_SITE_1": ["test-site-1"]}
        meta_key_site = ["site_id"]

        config.site.sites_to_report = sites_to_report
        config.auditor.site_meta_field = meta_key_site

        start_time_str = "2022-12-17 20:20:20+01:00"
        start_time = datetime.fromisoformat(start_time_str).replace(tzinfo=timezone.utc)

        client = FakeAuditorClient("fail")

        with pytest.raises(Exception) as pytest_error:
            get_records(config, client, start_time)
        assert pytest_error.type is RuntimeError

    def test_convert_to_seconds(self):
        cpu_time_name = "TotalCPU"
        cpu_time_unit = "seconds"

        conf = {}
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

        conf = {}
        conf["auditor"] = {
            "cpu_time_name": cpu_time_name,
            "cpu_time_unit": cpu_time_unit,
        }

        with pytest.raises(Exception) as pytest_error:
            convert_to_seconds(conf, 1100)
        assert pytest_error.type is ValueError
