import re
from argparse import Namespace
from datetime import date
from datetime import datetime as dt
from functools import reduce
from os.path import isfile
from typing import Iterator, List, Tuple, Union

import yaml

from .custom_types import Config as T_Config
from .custom_types import Keys
from .exceptions import (
    MalformedConfigEntryError,
    MissingConfigDependencyError,
    MissingConfigEntryError,
)
from .utils import extract_values


class Config(object):
    """Utility class to aggregate the configuration from CLI, file and defaults"""

    # default configuration
    _config: T_Config = {
        "interval": 900,
        "log_level": "INFO",
        "log_file": None,
        "history_file": None,
        "earliest_datetime": date.today().isoformat(),
        "query_type": "shell",
        "class_ads": frozenset(
            (
                "GlobalJobId",
                "ClusterId",
                "ProcId",
                "LastMatchTime",
                "EnteredCurrentStatus",
            )
        ),
    }

    def __init__(self, args: Namespace):
        self._config = type(self)._config.copy()
        with open(args.config) as f:
            file_config = yaml.safe_load(f)
            assert "class_ads" not in file_config, "config may not set 'class_ads'"

        self._config.update(file_config)
        self._config.update({k: v for k, v in args.__dict__.items() if v is not None})

        self._config["class_ads"] = self._config["class_ads"].union(
            extract_values("key", file_config["components"]),
            extract_values("key", file_config["meta"]),
        )
        self._verify()
        self._config["condor_timestamp"] = int(
            dt.fromisoformat(self.earliest_datetime).timestamp()
        )

    def __getattr__(self, attr: str):
        return self._config[attr]

    def get(self, attr: str, default=None):
        return self._config.get(attr, default)

    def _verify(self):
        """Verify presence and type of config items"""

        def get_nested(
            keys: Keys, config: T_Config = self._config
        ) -> Union[T_Config, int, str, List[T_Config], List[int], List[str], None]:
            """Provide `config[keys[0]]...[keys[-1]]` if present else `None`"""
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
            (["tls_config"], dict),
        ]:
            _cfg = get_nested(keys)
            if _cfg is None:
                raise MissingConfigEntryError(keys)
            if not isinstance(_cfg, _type):
                raise MalformedConfigEntryError(
                    keys, f"Must be of type {_type.__name__}"
                )
            if _type is list:
                assert isinstance(_cfg, list)  # For type checking
                if len(_cfg) == 0:
                    raise MalformedConfigEntryError(
                        keys, "Must contain at least one entry"
                    )

        # Check that certain config entries contain the required keys
        for keys in [["meta", "site"], ["components"]]:
            entries = get_nested(keys)
            assert isinstance(entries, list)  # For type checking
            for i, entry in enumerate(entries):
                if not isinstance(entry, dict):
                    raise MalformedConfigEntryError([*keys, i], "Must be a dictionary")
                if "name" not in entry or len(entry["name"].strip()) == 0:
                    if "matches" in entry:
                        try:
                            pattern = re.compile(entry["matches"])
                        except re.error as e:
                            raise MalformedConfigEntryError(
                                [*keys, i, "matches"],
                                "Is not a valid regular expression",
                            ) from e
                        if pattern.groups > 0:
                            continue
                    raise MalformedConfigEntryError(
                        [*keys, i],
                        "Must contain a non-empty string entry named 'name', or a regular "
                        "expression named 'matches' with a group matching the site name",
                    )

        # Check that certain config entries are lists of non-empty strings
        for keys in [["schedd_names"]]:
            entries = get_nested(keys)
            assert isinstance(entries, list)  # For type checking
            for i, entry in enumerate(entries):
                if not isinstance(entry, str) or len(entry.strip()) == 0:
                    raise MalformedConfigEntryError(
                        [*keys, i], "Must be a non-empty string"
                    )

        for keys in [["tls_config"]]:
            entries = get_nested(keys)
            if not isinstance(entries["use_tls"], bool):
                raise MalformedConfigEntryError(["use_tls"], "Must be a bool")
            if "use_tls" not in entries:
                raise MissingConfigDependencyError(["use_tls"])

            if entries["use_tls"]:
                certs = ["ca_cert_path", "client_cert_path", "client_key_path"]
                for cert_path in certs:
                    if not isinstance(entries[cert_path], str):
                        raise MalformedConfigEntryError([cert_path], "Must be a string")
                    if len(entries[cert_path].strip()) == 0:
                        raise MalformedConfigEntryError(
                            [cert_path], "Must be a non-empty string"
                        )
                    if cert_path not in entries:
                        raise MissingConfigDependencyError([cert_path])

        # If "job_status" is present, check that it is a list of integers
        if "job_status" in self._config:
            keys = ["job_status"]
            entries = get_nested(keys)
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
        if self._config["history_file"] is not None:
            if not isinstance(self._config["history_file"], str):
                raise MalformedConfigEntryError(["history_file"], "Must be a string")
            if not isfile(self._config["history_file"]):
                raise MalformedConfigEntryError(["history_file"], "Is not a file")

        if "query_type" in self._config:
            if self._config["query_type"] not in ("shell", "exec"):
                raise MalformedConfigEntryError(
                    ["query_type"], "Must be one of 'shell' or 'exec'"
                )

        if "constraint" in self._config:
            if not isinstance(self._config["constraint"], str):
                raise MalformedConfigEntryError(["constraint"], "Must be a string")

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
                    only_if = get_nested(keys[:-1])
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
