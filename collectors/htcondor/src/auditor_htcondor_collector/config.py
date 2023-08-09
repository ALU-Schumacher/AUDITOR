import yaml
import re

from argparse import Namespace
from functools import reduce
from typing import List, Tuple, Union, Iterator
from datetime import date, datetime as dt

from .utils import extract_values
from .exceptions import (
    MalformedConfigEntryError,
    MissingConfigEntryError,
    MissingConfigDependencyError,
)
from .custom_types import Keys, Config as T_Config


class Config(object):
    _config: T_Config = {
        "interval": 900,
        "log_level": "INFO",
        "log_file": None,
        "earliest_datetime": date.today().isoformat(),
        "class_ads": [
            "GlobalJobId",
            "ClusterId",
            "ProcId",
            "LastMatchTime",
            "EnteredCurrentStatus",
        ],
    }

    def __init__(self, args: Namespace):
        with open(args.config) as f:
            file = yaml.safe_load(f)

        self._config.update(file)
        self._config.update({k: v for k, v in args.__dict__.items() if v is not None})

        self._config["class_ads"] = list(
            set(self._config["class_ads"]).union(set(extract_values("key", file)))
        )
        self.check()
        self._config["condor_timestamp"] = int(
            dt.fromisoformat(self.earliest_datetime).timestamp()
        )

    def __getattr__(self, attr: str):
        return self._config[attr]

    def get(self, attr: str, default=None):
        return self._config.get(attr, default)

    def check(self):
        def _get(
            keys: Keys, config: T_Config = self._config
        ) -> Union[T_Config, int, str, List[T_Config], List[int], List[str], None]:
            try:
                return reduce(lambda d, k: d[k], keys, config)
            except KeyError:
                return None

        # Check for required keys and their types
        for keys, _type in [
            (["interval"], int),
            (["earliest_datetime"], str),
            (["log_level"], str),
            (["state_db"], str),
            (["record_prefix"], str),
            (["schedd_names"], list),
            (["meta"], dict),
            (["meta", "site"], list),
            (["components"], list),
        ]:
            _cfg = _get(keys)
            if _cfg is None:
                raise MissingConfigEntryError(keys)
            if not isinstance(_cfg, _type):
                raise MalformedConfigEntryError(
                    keys, f"Must be of type {_type.__name__}"
                )
            if _type == list:
                assert isinstance(_cfg, list)  # For type checking
                if len(_cfg) == 0:
                    raise MalformedConfigEntryError(
                        keys, "Must contain at least one entry"
                    )

        # Check that certain config entries contain the required keys
        for keys in [["meta", "site"], ["components"]]:
            entries = _get(keys)
            assert isinstance(entries, list)  # For type checking
            for i, entry in enumerate(entries):
                if not isinstance(entry, dict):
                    raise MalformedConfigEntryError([*keys, i], "Must be a dictionary")
                if "name" not in entry or len(entry["name"].strip()) == 0:
                    raise MalformedConfigEntryError(
                        [*keys, i],
                        "Must contain a non-empty string entry named 'name'",
                    )

        # Check that certain config entries are lists of non-empty strings
        for keys in [["schedd_names"]]:
            entries = _get(keys)
            assert isinstance(entries, list)  # For type checking
            for i, entry in enumerate(entries):
                if not isinstance(entry, str) or len(entry.strip()) == 0:
                    raise MalformedConfigEntryError(
                        [*keys, i], "Must be a non-empty string"
                    )

        # If "job_status" is present, check that it is a list of integers
        if "job_status" in self._config:
            keys = ["job_status"]
            entries = _get(keys)
            if not isinstance(entries, list):
                raise MalformedConfigEntryError(
                    keys, "Must be a list of job status entries (integers)"
                )
            for i, entry in enumerate(entries):
                if not isinstance(entry, int):
                    raise MalformedConfigEntryError([*keys, i], "Must be an integer")

        if "addr" in self._config:
            if not isinstance(self._config["addr"], str):
                raise MalformedConfigEntryError(["addr"], "Must be a string")
            if len(self._config["addr"].strip()) == 0:
                raise MalformedConfigEntryError(["addr"], "Must be a non-empty string")
            if "port" not in self._config:
                raise MissingConfigDependencyError(["port"], ["addr"])

        if "port" in self._config:
            if not isinstance(self._config["port"], int):
                raise MalformedConfigEntryError(["port"], "Must be an integer")
            if self._config["port"] < 0:
                raise MalformedConfigEntryError(["port"], "Must be a positive integer")
            if "addr" not in self._config:
                raise MissingConfigDependencyError(["addr"], ["port"])

        if "timeout" in self._config:
            if not isinstance(self._config["timeout"], int):
                raise MalformedConfigEntryError(["timeout"], "Must be an integer")
            if self._config["timeout"] < 0:
                raise MalformedConfigEntryError(
                    ["timeout"], "Must be a positive integer"
                )

        def _iter_config(
            keys: Keys = [], config: T_Config = self._config
        ) -> Iterator[Tuple[Keys, Union[str, int]]]:
            for key, value in config.items():
                _keys = [*keys, key]
                if isinstance(value, dict):
                    yield from _iter_config(keys=_keys, config=value)
                elif isinstance(value, list):
                    _list: T_Config = dict(enumerate(value))
                    yield from _iter_config(keys=_keys, config=_list)
                else:
                    yield _keys, value

        # Iterate over all config entries and check that they are valid
        for keys, value in _iter_config():
            if keys[-1] in ("name", "key", "pool"):
                if not isinstance(value, str):
                    raise MalformedConfigEntryError(keys, "Must be a string")
            elif keys[-1] == "matches":
                try:
                    if not isinstance(value, str):
                        raise MalformedConfigEntryError(
                            keys, "Must be a string containing a regular expression"
                        )
                    re.compile(value)
                except re.error:
                    raise MalformedConfigEntryError(
                        keys, "Must be a valid regular expression"
                    )
            elif keys[-1] == "earliest_datetime":
                try:
                    assert isinstance(value, str)  # For type checking
                    dt.fromisoformat(value)
                except (TypeError, ValueError):
                    raise MalformedConfigEntryError(
                        keys, "Must be a valid ISO 8601 datetime string"
                    )
            if len(keys) > 1:
                if keys[-2] == "only_if":
                    only_if = _get(keys[:-1])
                    if not isinstance(only_if, dict):
                        raise MalformedConfigEntryError(
                            keys[:-1],
                            "Must be a dictionary with keys 'key' and 'matches'",
                        )
                    if "key" not in only_if:
                        raise MalformedConfigEntryError(
                            keys[:-1], "Must contain the key 'key'"
                        )
                    if "matches" not in only_if:
                        raise MalformedConfigEntryError(
                            keys[:-1],
                            "Must contain the key 'matches' (a regular expression)",
                        )
