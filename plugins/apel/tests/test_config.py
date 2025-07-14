from datetime import datetime, timezone
from pathlib import Path, PurePath

import pyauditor
import pytest
import yaml
from pydantic import ValidationError

from auditor_apel_plugin.config import (
    AuditorConfig,
    ComponentField,
    Config,
    ConstantField,
    Field,
    Function,
    MetaField,
    NormalisedField,
    PluginConfig,
    RecordField,
    ScoreField,
    get_loaders,
)

test_dir = PurePath(__file__).parent


class TestConfig:
    def test_plugin_config(self):
        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        log_file = config.plugin.log_file
        assert log_file is None

        log_level = "DEBUG"
        log_file = "/var/log/test.log"
        time_json_path = "time.json"
        report_interval = 10

        plugin = PluginConfig(
            log_level=log_level,
            log_file=log_file,
            time_json_path=time_json_path,
            report_interval=report_interval,
        )

        assert plugin.log_level == "DEBUG"
        assert plugin.log_file == "/var/log/test.log"
        assert plugin.time_json_path == "time.json"
        assert plugin.report_interval == 10

        field_config = config.get_field_config()

        value = field_config.mandatory["CpuDuration"].name

        assert value == "TotalCPU"

        mandatory_fields = config.get_mandatory_fields()

        value = mandatory_fields["CpuDuration"].name

        assert value == "TotalCPU"

        optional_fields = config.get_optional_fields()

        value = optional_fields["GlobalUserName"].name

        assert value == "subject"

        all_fields = config.get_all_fields()

        value = all_fields["VO"].name

        assert value == "user"

        value = all_fields["CpuDuration"].name

        assert value == "TotalCPU"

        ip = "127.0.0.1"
        port = 8000
        timeout = 60
        site_meta_field = "site_id"
        use_tls = False

        auditorconfig = AuditorConfig(
            ip=ip,
            port=port,
            timeout=timeout,
            site_meta_field=site_meta_field,
            use_tls=use_tls,
        )

        assert auditorconfig.use_tls == use_tls

        use_tls = True

        with pytest.raises(Exception) as pytest_error:
            auditorconfig = AuditorConfig(
                ip=ip,
                port=port,
                timeout=timeout,
                site_meta_field=site_meta_field,
                use_tls=use_tls,
            )

        assert pytest_error.type is ValidationError

        ca_cert_path = "/test/path"

        with pytest.raises(Exception) as pytest_error:
            auditorconfig = AuditorConfig(
                ip=ip,
                port=port,
                timeout=timeout,
                site_meta_field=site_meta_field,
                use_tls=use_tls,
                ca_cert_path=ca_cert_path,
            )

        assert pytest_error.type is ValidationError

        client_cert_path = "/test/path"
        client_key_path = "/test/path"

        auditorconfig = AuditorConfig(
            ip=ip,
            port=port,
            timeout=timeout,
            site_meta_field=site_meta_field,
            use_tls=use_tls,
            ca_cert_path=ca_cert_path,
            client_cert_path=client_cert_path,
            client_key_path=client_key_path,
        )

        assert auditorconfig.ca_cert_path == ca_cert_path
        assert auditorconfig.client_cert_path == client_cert_path
        assert auditorconfig.client_key_path == client_key_path

    def test_get_value_default(self):
        class TestField(Field):
            attribute: str

        test_field = TestField(attribute="test")

        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        with pytest.raises(Exception) as pytest_error:
            test_field.get_value(record)

        assert pytest_error.type is NotImplementedError

    def test_get_value_meta_field(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        meta = pyauditor.Meta()
        meta.insert("meta_test", ["value"])
        record.with_meta(meta)

        meta_field = MetaField(name="meta_test")
        value = meta_field.get_value(record)

        assert value == "value"

    def test_get_value_meta_field_fail(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        meta = pyauditor.Meta()
        meta.insert("meta_test", ["value"])
        record.with_meta(meta)

        meta_field = MetaField(name="meta_test_missing")

        value = meta_field.get_value(record)

        assert value == "None"

        meta_field = MetaField(
            name="meta_test", function=Function(name="missing_function")
        )

        with pytest.raises(Exception) as pytest_error:
            meta_field.get_value(record)

        assert pytest_error.type is KeyError

    def test_meta_field_regex(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        meta = pyauditor.Meta()
        meta.insert("meta_test", ["aaavaluebbb"])
        record.with_meta(meta)

        meta_field = MetaField(name="meta_test", regex=r"(?<=aaa).*?\S(?=bbb)")

        value = meta_field.get_value(record)

        assert value == "value"

        meta_field = MetaField(name="meta_test", regex=r"(?<=aaa).*?\S(?=ccc)")

        value = meta_field.get_value(record)

        assert value == "None"

    def test_get_value_component_field(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        component = pyauditor.Component(name="test_component", amount=1000)
        record.with_component(component)

        component_field = ComponentField(name="test_component")
        value = component_field.get_value(record)

        assert value == 1000

        component_field = ComponentField(
            name="test_component",
            divide_by=1000,
        )
        value = component_field.get_value(record)

        assert value == 1

        component_field = ComponentField(name="test_component_2")

        with pytest.raises(Exception) as pytest_error:
            value = component_field.get_value(record)

        assert pytest_error.type is ValueError

    def test_get_value_score_field(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        score = pyauditor.Score(name="test_score", value=2.5)
        component = pyauditor.Component(name="test_component", amount=1000)
        component.with_score(score)
        record.with_component(component)

        score_field = ScoreField(
            name="test_score",
            component_name="test_component",
        )
        value = score_field.get_value(record)

        assert value == 2.5

        score_field = ScoreField(
            name="test_score",
            component_name="test_component_2",
        )

        with pytest.raises(Exception) as pytest_error:
            value = score_field.get_value(record)

        assert pytest_error.type is ValueError

        score_field = ScoreField(
            name="test_score_2",
            component_name="test_component",
        )

        with pytest.raises(Exception) as pytest_error:
            value = score_field.get_value(record)

        assert pytest_error.type is ValueError

    def test_get_value_normalised_field(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        score = pyauditor.Score(name="test_score", value=2.5)
        component = pyauditor.Component(name="test_component", amount=1000)
        component.with_score(score)
        record.with_component(component)

        score_field = ScoreField(
            name="test_score",
            component_name="test_component",
        )
        component_field = ComponentField(name="test_component")

        normalised_field = NormalisedField(
            base_value=component_field, score=score_field
        )
        value = normalised_field.get_value(record)

        assert value == 2500.0

    def test_get_value_normalised_field_fail(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        score = pyauditor.Score(name="test_score", value=2.5)
        component = pyauditor.Component(name="test_component", amount=1000)
        component.with_score(score)
        record.with_component(component)

        record_field = RecordField(name="record_id")

        score_field = ScoreField(
            name="test_score",
            component_name="test_component",
        )

        normalised_field = NormalisedField(base_value=record_field, score=score_field)

        with pytest.raises(Exception) as pytest_error:
            normalised_field.get_value(record)

        assert pytest_error.type is TypeError

    def test_get_value_constant_field(self):
        constant_field = ConstantField(value="test")
        value = constant_field.get_value()

        assert value == "test"

        constant_field = ConstantField(value=15)
        value = constant_field.get_value()

        assert value == 15

    def test_get_value_record_field(self):
        record = pyauditor.Record(
            "record_id_123",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        record_field = RecordField(name="record_id")

        value = record_field.get_value(record)

        assert value == "record_id_123"

        record_field = RecordField(name="start_time", modify="month")

        value = record_field.get_value(record)

        assert value == 3

    def test_get_value_record_field_fail(self):
        record = pyauditor.Record(
            "record_id_123",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        record_field = RecordField(name="missing")

        with pytest.raises(Exception) as pytest_error:
            record_field.get_value(record)

        assert pytest_error.type is AttributeError

        record_field = RecordField(name="start_time", modify="missing")

        with pytest.raises(Exception) as pytest_error:
            record_field.get_value(record)

        assert pytest_error.type is AttributeError

    def test_loaders(self):
        test_yaml = """
                    !ComponentField
                      name: test_field
                    """

        config = yaml.load(test_yaml, Loader=get_loaders())
        value = config.name

        assert value == "test_field"

    def test_vo_mapping(self):
        record = pyauditor.Record(
            "record_id",
            datetime(1984, 3, 3, 0, 0, 0).astimezone(tz=timezone.utc),
        )

        vo_dict = {"atlpr": "atlas", "atlsg": "ops", "ops": "ops"}

        meta = pyauditor.Meta()
        meta.insert("user", ["atlpr000"])
        record.with_meta(meta)

        meta_field = MetaField(
            name="user", function=Function(name="vo_mapping", parameters=vo_dict)
        )
        value = meta_field.get_value(record)

        assert value == "atlas"

        with open(Path.joinpath(test_dir, "test_config.yml"), "r") as f:
            config: Config = yaml.load(f, Loader=get_loaders())

        value = config.summary_fields.mandatory["VO"].get_value(record)

        assert value == "atlas"

        meta = pyauditor.Meta()
        meta.insert("user", ["atlsg000"])
        record.with_meta(meta)

        value = config.summary_fields.mandatory["VO"].get_value(record)

        assert value == "ops"

        meta = pyauditor.Meta()
        meta.insert("user", ["ops000"])
        record.with_meta(meta)

        value = config.summary_fields.mandatory["VO"].get_value(record)

        assert value == "ops"

        meta = pyauditor.Meta()
        meta.insert("user", ["ilc000"])
        record.with_meta(meta)

        value = config.summary_fields.mandatory["VO"].get_value(record)

        assert value == "None"
