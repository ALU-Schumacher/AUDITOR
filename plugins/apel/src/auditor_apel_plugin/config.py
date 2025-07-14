#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import logging
import re
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, Union

import yaml
from pyauditor import Record
from pydantic import BaseModel, model_validator

from .utility import vo_mapping

logger = logging.getLogger("apel_plugin")


class MessageType(Enum):
    summaries = "summaries"
    sync = "sync"

    @classmethod
    def _missing_(cls, value):
        return cls.summaries


class Configurable(BaseModel):
    @classmethod
    def from_yaml(cls, loader: yaml.SafeLoader, node: yaml.nodes.MappingNode):
        mapping = loader.construct_mapping(node, deep=True)
        string_mapping = {str(k): v for k, v in mapping.items()}

        return cls(**string_mapping)


class Function(Configurable):
    name: str
    parameters: Any = None


class PluginConfig(Configurable):
    log_level: str
    log_file: Optional[str] = None
    time_json_path: str
    report_interval: int


class SiteConfig(Configurable):
    sites_to_report: Dict[str, List[str]]


class AuditorConfig(Configurable):
    ip: str
    port: int
    timeout: int
    site_meta_field: Union[str, List[str]]
    use_tls: bool
    ca_cert_path: Optional[str] = None
    client_cert_path: Optional[str] = None
    client_key_path: Optional[str] = None

    @model_validator(mode="after")
    def check_tls_config(self):
        if self.use_tls:
            missing_parameters = [
                parameter_name
                for parameter_name, value in {
                    "ca_cert_path": self.ca_cert_path,
                    "client_cert_path": self.client_cert_path,
                    "client_key_path": self.client_key_path,
                }.items()
                if value is None
            ]
            if missing_parameters:
                missing_parameters_str = ", ".join(missing_parameters)
                raise ValueError(
                    f"Parameters {missing_parameters_str} are required if use_tls: True"
                )
        return self


class MessageConfig(Configurable):
    host: str
    port: int
    client_cert: str
    client_key: str
    project: str
    topic: str
    timeout: int
    retry: int


class Field(Configurable):
    def get_value(self, record: Optional[Record]) -> Union[str, int, float]:
        raise NotImplementedError()


class FieldConfig(Configurable):
    mandatory: Dict[str, Field]
    optional: Dict[str, Field] = {}

    @model_validator(mode="after")
    def check_mandatory_fields(self):
        mandatory_fields = [
            "Processors",
            "VO",
            "SubmitHost",
            "CpuDuration",
            "NormalisedCpuDuration",
            "NormalisedWallDuration",
        ]
        missing_fields = [
            field_name
            for field_name in mandatory_fields
            if field_name not in list(self.mandatory.keys())
        ]
        if missing_fields:
            missing_fields_str = ", ".join(missing_fields)
            raise ValueError(f"Fields {missing_fields_str} are mandatory")
        return self


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
    function: Optional[Function] = None

    def get_value(self, record: Record) -> Union[str, int, float]:
        function_dict: Dict[str, Callable[[str, Any], Union[str, int, float]]] = {
            "vo_mapping": vo_mapping
        }

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
                function = function_dict[self.function.name]
            except KeyError:
                logger.critical(
                    f"Function {self.function.name} not found in dictionary of allowed "
                    "functions"
                )
                raise
            value = function(value, self.function.parameters)
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
    base_value: Union[ComponentField, RecordField] = RecordField(name="runtime")
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


class ConstantField(Field):
    value: Union[str, int, float]

    def get_value(self, record: Optional[Record] = None) -> Union[str, int, float]:
        return self.value


class Config(Configurable):
    plugin: PluginConfig
    site: SiteConfig
    auditor: AuditorConfig
    messaging: MessageConfig
    summary_fields: FieldConfig

    def get_field_config(self) -> FieldConfig:
        if self.summary_fields is not None:
            field_config = self.summary_fields
        else:
            logger.critical("summary_fields missing in config!")
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
    loader.add_constructor("!RecordField", RecordField.from_yaml)
    return loader
