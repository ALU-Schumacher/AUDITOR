import yaml
import re

from functools import reduce

from utils import extract_values
from exceptions import MalformedConfigEntryError, MissingConfigEntryError


class Config(object):
    _config = {
        "interval": 900,
        "log_level": "INFO",
        "log_file": None,
        "class_ads": [
            "GlobalJobId",
            "ClusterId",
            "ProcId",
            "LastMatchTime",
            "EnteredCurrentStatus",
        ],
    }

    def __init__(self, args):
        with open(args.config) as f:
            file = yaml.safe_load(f)

        self._config.update(file)
        self._config.update({k: v for k, v in args.__dict__.items() if v is not None})

        self._config["class_ads"] = list(
            set(self._config["class_ads"]).union(set(extract_values("key", file)))
        )
        self.check()

    def __getattr__(self, attr):
        return self._config[attr]

    def get(self, attr, default=None):
        return self._config.get(attr, default)

    def check(self):
        def _get(keys, config=self._config):
            try:
                return reduce(lambda d, k: d[k], keys, config)
            except KeyError:
                return None

        # Check for required keys and their types
        for keys, _type in [
            (["addr"], str),
            (["port"], int),
            (["interval"], int),
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
            if _type == list and len(_cfg) == 0:
                raise MalformedConfigEntryError(keys, "Must not be empty list")

        # Check that certain config entries contain the required keys
        for keys in [["meta", "site"], ["components"]]:
            for i, entry in enumerate(_get(keys)):  # type: ignore
                if "name" not in entry or len(entry["name"].strip()) == 0:
                    raise MalformedConfigEntryError(
                        keys + [i],
                        "Must contain the key 'name' and it must not be empty",
                    )

        # Check that certain config entries are lists of non-empty strings
        for keys in [["schedd_names"]]:
            for i, entry in enumerate(_get(keys)):  # type: ignore
                if not isinstance(entry, str) or len(entry.strip()) == 0:
                    raise MalformedConfigEntryError(
                        keys + [i], "Must be a non-empty string"
                    )

        # If "job_status" is present, check that it is a list of integers
        if "job_status" in self._config:
            keys = ["job_status"]
            entries = _get(keys)
            if not isinstance(entries, list):
                raise MalformedConfigEntryError(
                    keys, "Must be a list of job status entries"
                )
            for i, entry in enumerate(entries):
                if not isinstance(entry, int):
                    raise MalformedConfigEntryError(keys + [i], "Must be an integer")

        def _iter_config(keys=[], config=self._config):
            for key, value in config.items():
                _keys = keys + [key]
                if isinstance(value, dict):
                    yield from _iter_config(keys=_keys, config=value)
                elif isinstance(value, list):
                    _list = dict(enumerate(value))
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
                    re.compile(value)
                except re.error:
                    raise MalformedConfigEntryError(
                        keys, "Must be a valid regular expression"
                    )
            if len(keys) > 1:
                if keys[-2] == "only_if":
                    only_if = _get(keys[:-1])
                    if "key" not in only_if:  # type: ignore
                        raise MalformedConfigEntryError(
                            keys[:-1], "Must contain the key 'key'"
                        )
                    if "matches" not in only_if:  # type: ignore
                        raise MalformedConfigEntryError(
                            keys[:-1], "Must contain the key 'matches'"
                        )
