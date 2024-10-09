from datetime import datetime, timezone
from pathlib import Path, PurePath

import pyauditor
import pytest
import yaml
from pydantic import ValidationError

from auditor_apel_plugin.config import (
    ComponentField,
    Config,
    ConstantField,
    Field,
    MessageType,
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

        log_level = "DEBUG"
        time_json_path = "time.json"
        report_interval = 10
        message_type = "summaries"

        plugin = PluginConfig(
            log_level=log_level,
            time_json_path=time_json_path,
            report_interval=report_interval,
            message_type=message_type,
        )

        assert plugin.log_level == "DEBUG"
        assert plugin.time_json_path == "time.json"
        assert plugin.report_interval == 10
        assert plugin.message_type is MessageType("summaries")

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

        assert value == "voms"

        value = all_fields["CpuDuration"].name

        assert value == "TotalCPU"

        message_type = "something_else"

        with pytest.raises(Exception) as pytest_error:
            plugin = PluginConfig(
                log_level=log_level,
                time_json_path=time_json_path,
                report_interval=report_interval,
                message_type=message_type,
            )
        assert pytest_error.type is ValidationError

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

        meta_field = MetaField(name="meta_test", function="missing_function")

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
