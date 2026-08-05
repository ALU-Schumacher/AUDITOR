import json
import os
import subprocess
from datetime import datetime, timezone
from importlib.metadata import version
from pathlib import PurePath

import pytest
import yaml
from auditor_apel_plugin.config import Config, get_loaders
from auditor_apel_plugin.core import (
    construct_infrastructure,
    create_time_json,
    get_begin_current_month,
    get_begin_previous_month,
    get_records,
    get_report_time,
    get_time_json,
    sign_msg,
    update_time_json,
)

import pyauditor

test_dir = PurePath(__file__).parent


class FakeAuditorClient:
    def __init__(self, test_case=""):
        self.test_case = test_case

    def advanced_query(self, start_time):
        if self.test_case == "pass":
            return "good"
        if self.test_case == "fail":
            raise RuntimeError("RuntimeError")


class TestAuditorApelPlugin:
    def test_get_begin_previous_month(self):
        time_a = datetime(2022, 10, 23, 12, 23, 55, tzinfo=timezone.utc)
        time_b = datetime(1970, 1, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_previous_month(time_a)
        assert result == datetime(2022, 9, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_previous_month(time_b)
        assert result == datetime(1969, 12, 1, 00, 00, 00, tzinfo=timezone.utc)

    def test_get_begin_current_month(self):
        time_a = datetime(2022, 10, 23, 12, 23, 55, tzinfo=timezone.utc)
        time_b = datetime(1970, 1, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_current_month(time_a)
        assert result == datetime(2022, 10, 1, 00, 00, 00, tzinfo=timezone.utc)

        result = get_begin_current_month(time_b)
        assert result == datetime(1970, 1, 1, 00, 00, 00, tzinfo=timezone.utc)

    def test_create_time_json(self):
        path = "/tmp/test.json"

        result = create_time_json(path)

        assert result["last_report_time"] == "1970-01-01T00:00:00+00:00"

        with open("/tmp/test.json", encoding="utf-8") as f:
            result = json.load(f)

        os.remove(path)

        assert result["last_report_time"] == "1970-01-01T00:00:00+00:00"

    def test_create_time_json_fail(self):
        path = "/home/nonexistent/55/abc/time.db"

        with pytest.raises(Exception) as pytest_error:
            create_time_json(path)

        assert pytest_error.type is FileNotFoundError

    def test_get_time_json(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/nonexistent_55_abc_time.db"

        config.plugin.time_json_path = path

        result = get_time_json(config)
        os.remove(path)

        assert result["last_report_time"] == "1970-01-01T00:00:00+00:00"

        create_time_json(path)
        result = get_time_json(config)
        os.remove(path)

        assert result["last_report_time"] == "1970-01-01T00:00:00+00:00"

    def test_sign_msg(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        config.messaging.client_cert = test_dir.joinpath("test_cert.cert")
        config.messaging.client_key = test_dir.joinpath("test_key.key")

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
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        config.messaging.client_cert = "tests/nodir/test_cert.cert"
        config.messaging.client_key = "tests/no/dir/test_key.key"

        with pytest.raises(Exception) as pytest_error:
            sign_msg(config, "test")

        assert pytest_error.type is FileNotFoundError

        config.messaging.client_cert = str(test_dir.joinpath("test_cert.cert"))
        config.messaging.client_key = str(test_dir.joinpath("test_key.key"))

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
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"
        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

        result = get_report_time(time_dict)

        initial_report_time = datetime(1970, 1, 1, 0, 0, 0, tzinfo=timezone.utc)

        assert result == initial_report_time

        new_report_time = datetime(2023, 2, 10, 11, 13, 45, tzinfo=timezone.utc)

        update_time_json(
            config,
            time_dict,
            new_report_time,
        )

        result = get_report_time(time_dict)
        os.remove(path)
        assert result == new_report_time

    def test_update_time_json(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"

        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

        report_time_list = [
            datetime(1993, 4, 4, 0, 0, 0, tzinfo=timezone.utc),
            datetime(2100, 8, 19, 14, 16, 11, tzinfo=timezone.utc),
            datetime(1887, 2, 27, 0, 11, 31, tzinfo=timezone.utc),
        ]

        for report_time in report_time_list:
            update_time_json(config, time_dict, report_time)

            with open(path, encoding="utf-8") as f:
                time_dict_test = json.load(f)
                last_report_time = time_dict_test["last_report_time"]

            assert last_report_time == report_time.isoformat()

        os.remove(path)

    def test_update_time_json_fail(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        path = "/tmp/test.json"

        config.plugin.time_json_path = path

        time_dict = create_time_json(path)

        report_time = datetime(1993, 4, 4, 0, 0, 0, tzinfo=timezone.utc)

        os.remove(path)

        path_new = "/dfghdfh/test.json"
        config.plugin.time_json_path = path_new

        with pytest.raises(Exception) as pytest_error:
            update_time_json(config, time_dict, report_time)

        assert pytest_error.type is FileNotFoundError

    def test_get_records(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        client = FakeAuditorClient("pass")

        config.auditor.site_meta_field = ["site_id"]

        clusters = ["test-site-1"]

        start_time_str = "2022-12-17 20:20:20+01:00"
        start_time = datetime.fromisoformat(start_time_str).replace(tzinfo=timezone.utc)

        result = get_records(config, client, start_time, clusters)
        assert "".join(result) == "good"

    def test_get_records_fail(self):
        with open(test_dir.joinpath("test_config.yml")) as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        client = FakeAuditorClient("fail")

        config.auditor.site_meta_field = ["site_id"]

        clusters = ["test-site-1"]

        start_time_str = "2022-12-17 20:20:20+01:00"
        start_time = datetime.fromisoformat(start_time_str).replace(tzinfo=timezone.utc)

        with pytest.raises(Exception) as pytest_error:
            get_records(config, client, start_time, clusters)
        assert pytest_error.type is RuntimeError

    def test_construct_infrastructure(self):
        compute_element = "ARC"
        plugin_version = version("auditor_apel_plugin")

        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        meta = pyauditor.Meta()
        meta.insert("meta_test", ["value"])
        record.with_meta(meta)

        infrastructure = construct_infrastructure(
            plugin_version, compute_element, record
        )

        assert infrastructure == f"AUDITOR_{plugin_version}-ARC-UNKNOWN"

        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        meta = pyauditor.Meta()
        meta.insert("collector_type", ["HTCondor", "1.0.0"])
        record.with_meta(meta)

        infrastructure = construct_infrastructure(
            plugin_version, compute_element, record
        )

        assert infrastructure == f"AUDITOR_{plugin_version}-ARC-HTCondor"
