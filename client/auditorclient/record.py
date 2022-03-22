"""Records"""

from __future__ import annotations  # not necessary in 3.10
import json
from .errors import InsufficientParametersError


class Components:
    def __init__(self):
        self._components = []

    def __str__(self) -> str:
        return self._components.__str__()

    def __eq__(self, other: Components) -> bool:
        return self._components == other._components

    def add_component(self, name: str, amount: int, factor: float) -> Components:
        self._components.append({"name": name, "amount": amount, "factor": factor})
        return self

    def get(self) -> [dict]:
        return self._components


class Record:
    def __init__(
        self,
        record_id: str = None,
        site_id: str = None,
        user_id: str = None,
        group_id: str = None,
        components: Components = None,
        json_str: str = None,
    ):
        if (
            record_id is None
            or site_id is None
            or user_id is None
            or group_id is None
            or components is None
        ) and json_str is None:
            raise InsufficientParametersError
        if json_str is None:
            self._record_id = record_id
            self._site_id = site_id
            self._user_id = user_id
            self._group_id = group_id
            self._components = components
            self._start_time = None
            self._stop_time = None
            #  self._runtime = None
        else:
            d = json.loads(json_str)
            c = Components()
            for comp in d["components"]:
                c.add_component(comp["name"], comp["amount"], comp["factor"])
            self._record_id = d["record_id"]
            self._site_id = d["site_id"]
            self._user_id = d["user_id"]
            self._group_id = d["group_id"]
            self._components = c
            self._start_time = d["start_time"]
            self._stop_time = d["stop_time"]
            #  self._runtime = d["runtime"]

    def __str__(self) -> str:
        return self.as_dict().__str__()

    def __eq__(self, other: Record) -> bool:
        return (
            self._record_id == other._record_id
            and self._site_id == other._site_id
            and self._user_id == other._user_id
            and self._group_id == other._group_id
            and self._components == other._components
            and self._start_time == other._start_time
            and self._stop_time == other._stop_time
        )

    def with_start_time(self, start_time: str) -> Record:
        self._start_time = start_time
        return self

    def with_stop_time(self, stop_time: str) -> Record:
        self._stop_time = stop_time
        return self

    #  def with_runtime(self, runtime: str) -> Record:
    #      self._runtime = runtime
    #      return self

    def record_id(self) -> str:
        return self._record_id

    def site_id(self) -> str:
        return self._site_id

    def as_dict(self) -> dict:
        return {
            "record_id": self._record_id,
            "site_id": self._site_id,
            "user_id": self._user_id,
            "group_id": self._group_id,
            "components": self._components.get(),
            "start_time": self._start_time,
            "stop_time": self._stop_time,
        }

    def as_json(self) -> str:
        return json.dumps(self.as_dict())
