#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
import yaml
from datetime import datetime
from enum import Enum
from pydantic import BaseModel
from typing import Optional, Union, Dict, List, Callable
from pyauditor import Record
import re

logger = logging.getLogger("apel_plugin")


class MessageType(Enum):
    summaries = "summaries"
    individual_jobs = "individual_jobs"
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
            logger.critical(
                f"Component {self.name} not found in record {record.record_id}"
            )
            raise ValueError

        if self.divide_by is not None:
            value = round(value / self.divide_by)

        return value


class MetaField(Field):
    name: str
    regex: Optional[str] = None
    function: Optional[str] = None

    def get_value(self, record: Record) -> Union[str, int, float]:
        function_dict: Dict[str, Callable[[str], Union[str, int, float]]] = {}

        try:
            value = record.meta.get(self.name)[0]
        except TypeError:
            logger.warning(f"Meta {self.name} not found in record {record.record_id}")
            return "None"

        if self.regex is not None:
            re_match = re.search(self.regex, value)

            if re_match is not None:
                value = re_match.group(0)
                return value
            else:
                logger.warning(f"Pattern {self.regex} not found in {value}")
                return "None"
        elif self.function is not None:
            try:
                function = function_dict[self.function]
            except KeyError:
                logger.critical(
                    f"Function {self.function} not found in dictionary of allowed "
                    "functions"
                )
                raise
            value = function(value)
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
            logger.critical(
                f"Component {self.name} not found in record {record.record_id}"
            )
            raise ValueError

        for s in scores:
            if s.name == self.name:
                value = s.value
                break

        if value is None:
            logger.critical(
                f"Score {self.name} not found in component {self.component_name} of "
                f"record {record.record_id}"
            )
            raise ValueError

        return value


class RecordField(Field):
    name: str
    modify: Optional[str] = None

    def get_value(self, record: Record) -> Union[str, int]:
        try:
            value = getattr(record, self.name)
        except AttributeError:
            logger.critical(
                f"Record {record.record_id} does not have attribute {self.name}"
            )
            raise

        if self.modify is not None:
            try:
                value = getattr(value, self.modify)
            except AttributeError:
                logger.critical(
                    f"Value {value} of type {type(value)} does not have attribute "
                    f"{self.name}"
                )
                raise

        return value


class NormalisedField(Field):
    base_value: Union[ComponentField, RecordField]
    score: ScoreField

    def get_value(self, record: Record) -> int:
        base_value = self.base_value.get_value(record)
        score_value = self.score.get_value(record)

        if isinstance(base_value, str):
            logger.critical(
                f"base_value of NormalisedField is a string: {base_value}. "
                "Multiplication not possible!"
            )
            raise TypeError
        value = round(base_value * score_value)

        return value


class NormalisedWallDurationField(NormalisedField):
    base_value: RecordField = RecordField(name="runtime")


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
    individual_job_fields: Optional[FieldConfig] = None

    def get_field_config(self) -> FieldConfig:
        if self.plugin.message_type == MessageType.summaries:
            if self.summary_fields is not None:
                field_config = self.summary_fields
            else:
                logger.critical("summary_fields missing in config!")
                raise ValueError
        elif self.plugin.message_type == MessageType.individual_jobs:
            if self.individual_job_fields is not None:
                field_config = self.individual_job_fields
            else:
                logger.critical("individual_job_fields missing in config!")
                raise ValueError

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
