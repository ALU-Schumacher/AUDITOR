#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import yaml
from datetime import datetime
from enum import Enum
from pydantic import BaseModel
from typing import Optional, Union, Dict, List
from pyauditor import Record
import re


class MessageType(Enum):
    summaries = "summaries"
    single_jobs = "single_jobs"
    sync = "sync"


class Configurable(BaseModel):
    @classmethod
    def from_yaml(cls, loader: yaml.SafeLoader, node: yaml.nodes.MappingNode):
        return cls(**loader.construct_mapping(node, deep=True))


class PluginConfig(Configurable):
    log_level: str
    time_json_path: str
    report_interval: int
    message_type: MessageType


class SiteConfig(Configurable):
    publish_since: datetime
    sites_to_report: Dict[str, List[str]]


class AuditorConfig(Configurable):
    ip: str
    port: int
    timeout: int
    site_meta_field: str


class AuthConfig(Configurable):
    auth_url: str
    ams_url: str
    client_cert: str
    client_key: str
    ca_path: str
    verify_ca: bool


class Field(Configurable):
    datatype_in_message: str

    def get_value(self, record: Optional[Record]) -> Union[str, int, float]:
        raise NotImplementedError()


class FieldConfig(Configurable):
    mandatory: Dict[str, Field]
    optional: Dict[str, Field] = {}


class ComponentField(Field):
    name: str
    divide_by: Optional[int] = None

    def get_value(self, record: Record) -> int:
        components = record.components
        value = None

        for c in components:
            if c.name == self.name:
                value = c.amount
                break

        if value is None:
            raise ValueError(
                f"Component {self.name} not found in record {record.record_id}"
            )

        if self.divide_by is not None:
            value = round(value / self.divide_by)

        return value


class MetaField(Field):
    name: str
    regex: Optional[str] = None
    function: Optional[str] = None

    def get_value(self, record: Record) -> str:
        try:
            value = record.meta.get(self.name)[0]
        except TypeError:
            print(f"WARNING: Meta {self.name} not found in record {record.record_id}")
            return "None"

        if self.regex is not None:
            re_match = re.search(self.regex, value)

            if re_match is not None:
                value = re_match.group(0)
                return value
            else:
                print(f"WARNING: Pattern {self.regex} not found in {value}")
                return "None"
        elif self.function is not None:
            value = globals()[self.function](value)
            return value

        return value


class ScoreField(Field):
    name: str
    component_name: str

    def get_value(self, record: Record) -> float:
        components = record.components
        scores = None
        value = None

        for c in components:
            if c.name == self.component_name:
                scores = c.scores
                break

        if scores is None:
            raise ValueError(
                f"Component {self.name} not found in record {record.record_id}"
            )

        for s in scores:
            if s.name == self.name:
                value = s.value
                break

        if value is None:
            raise ValueError(
                f"Score {self.name} not found in component {self.component_name} of "
                f"record {record.record_id}"
            )

        return value


class RecordField(Field):
    name: str
    modify: Optional[str] = None

    def get_value(self, record: Record) -> Union[str, int]:
        value = getattr(record, self.name)

        if self.modify is not None:
            value = getattr(value, self.modify)

        return value


class NormalisedField(Field):
    base_value: Union[ComponentField, RecordField]
    score: ScoreField

    def get_value(self, record: Record) -> float:
        base_value = self.base_value.get_value(record)
        score_value = self.score.get_value(record)

        if isinstance(base_value, str):
            raise TypeError(
                f"base_value of NormalisedField is a string: {base_value}. "
                "Multiplication not possible!"
            )
        value = base_value * score_value

        return value


class NormalisedWallDurationField(NormalisedField):
    base_value: RecordField = RecordField(name="runtime", datatype_in_message="INT")


class ConstantField(Field):
    value: Union[str, int, float]

    def get_value(self, record: Optional[Record] = None) -> Union[str, int, float]:
        return self.value


class Config(Configurable):
    plugin: PluginConfig
    site: SiteConfig
    auditor: AuditorConfig
    authentication: AuthConfig
    summary_fields: Optional[FieldConfig] = None
    single_job_fields: Optional[FieldConfig] = None

    def get_field_config(self) -> FieldConfig:
        if self.plugin.message_type == MessageType.summaries:
            if self.summary_fields is not None:
                field_config = self.summary_fields
            else:
                raise ValueError("summary_fields missing in config!")
        elif self.plugin.message_type == MessageType.single_jobs:
            if self.single_job_fields is not None:
                field_config = self.single_job_fields
            else:
                raise ValueError("single_job_fields missing in config!")

        return field_config

    def get_mandatory_fields(self) -> Dict[str, Field]:
        return self.get_field_config().mandatory

    def get_optional_fields(self) -> Dict[str, Field]:
        return self.get_field_config().optional

    def get_all_fields(self) -> Dict[str, Field]:
        mandatory_dict = self.get_mandatory_fields()
        optional_dict = self.get_optional_fields()

        all_fields_dict = {**mandatory_dict, **optional_dict}

        return all_fields_dict


def get_loaders():
    loader = yaml.SafeLoader
    loader.add_constructor("!Config", Config.from_yaml)
    loader.add_constructor("!ComponentField", ComponentField.from_yaml)
    loader.add_constructor("!MetaField", MetaField.from_yaml)
    loader.add_constructor("!ScoreField", ScoreField.from_yaml)
    loader.add_constructor("!ConstantField", ConstantField.from_yaml)
    loader.add_constructor("!NormalisedField", NormalisedField.from_yaml)
    loader.add_constructor(
        "!NormalisedWallDurationField", NormalisedWallDurationField.from_yaml
    )
    loader.add_constructor("!RecordField", RecordField.from_yaml)
    return loader
