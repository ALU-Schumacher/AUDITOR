"""Tests for the JSON-based `condor_history` parsing and checkpoint handling.

These tests cover the fix for the comma-delimited autoformat parsing bug:
a ClassAd value containing a comma (e.g. an X.509 subject with
``O=Fermi Forward Discovery Group, LLC``) must no longer shift columns and
corrupt ``ProcId``/the stored checkpoint.

`pyauditor` is a compiled dependency that is not needed for the parsing logic,
so it is stubbed out before importing the collector module.
"""

import json
import sys
import types
from argparse import Namespace

import pytest

# ---------------------------------------------------------------------------
# Stub out pyauditor so collector.py can be imported without the compiled dep.
# ---------------------------------------------------------------------------
_pyauditor = types.ModuleType("pyauditor")
for _name in (
    "AuditorClient",
    "AuditorClientBuilder",
    "Component",
    "Meta",
    "Record",
    "Score",
):
    setattr(_pyauditor, _name, type(_name, (), {}))
sys.modules.setdefault("pyauditor", _pyauditor)

from auditor_htcondor_collector.collector import (  # noqa: E402
    CondorHistoryCollector,
    _is_valid_job_id,
)
from auditor_htcondor_collector.config import Config  # noqa: E402

# A real subject DN that nonetheless contains a literal comma in the O= value.
FERMI_DN = (
    "/DC=org/DC=incommon/C=US/ST=Illinois/"
    "O=Fermi Forward Discovery Group, LLC/CN=pilot-cmssrv2426.fnal.gov"
)

CONFIG_YAML = f"""
state_db: "{{state_db}}"
record_prefix: htcondor-test
schedd_names:
  - schedd@test
meta:
  user:
    key: Owner
  subject:
    key: x509UserProxySubject
  voms:
    key: x509UserProxyFirstFQAN
  site:
    - name: "TEST-SITE"
components:
  - name: "Cores"
    key: "CpusProvisioned"
  - name: "CPUTime"
    key: "RemoteUserCpu+RemoteSysCpu"   # expression key (DESY style)
tls_config:
  use_tls: False
"""


@pytest.fixture
def collector(tmp_path):
    cfg_file = tmp_path / "config.yaml"
    cfg_file.write_text(CONFIG_YAML.format(state_db=str(tmp_path / "state.db")))
    config = Config(Namespace(config=str(cfg_file)))
    # Bypass __init__ (which builds the auditor client); we only need parsing.
    c = CondorHistoryCollector.__new__(CondorHistoryCollector)
    c.config = config
    import logging

    c.logger = logging.getLogger("test")
    return c


def _hist_json(*ads):
    """Render ads as `condor_history -json` would (a JSON array)."""
    return json.dumps(list(ads)).encode("utf-8")


def test_comma_in_subject_does_not_shift_columns(collector):
    ad = {
        "GlobalJobId": "ce05#20096.0#1700000000",
        "ClusterId": 20096,
        "ProcId": 0,
        "Owner": "cms001",
        "x509UserProxySubject": FERMI_DN,
        "x509UserProxyFirstFQAN": "/cms/Role=pilot/Capability=NULL",
        "CpusProvisioned": 1,
        "RemoteUserCpu": 100,
        "RemoteSysCpu": 23,
    }
    jobs = collector._parse_history(_hist_json(ad))
    assert len(jobs) == 1
    job = jobs[0]
    # The whole point: ProcId stays an int, and the comma-bearing DN is intact.
    assert job["ClusterId"] == 20096 and isinstance(job["ClusterId"], int)
    assert job["ProcId"] == 0 and isinstance(job["ProcId"], int)
    assert job["x509UserProxySubject"] == FERMI_DN
    assert job["x509UserProxyFirstFQAN"] == "/cms/Role=pilot/Capability=NULL"


def test_expression_key_is_evaluated(collector):
    ad = {
        "GlobalJobId": "ce05#1.0#1",
        "ClusterId": 1,
        "ProcId": 0,
        "RemoteUserCpu": 100,
        "RemoteSysCpu": 23,
    }
    job = collector._parse_history(_hist_json(ad))[0]
    assert job["RemoteUserCpu+RemoteSysCpu"] == 123


def test_undefined_attributes_are_dropped(collector):
    ad = {
        "GlobalJobId": "ce05#2.0#2",
        "ClusterId": 2,
        "ProcId": 0,
        "Owner": "undefined",
        "x509UserProxySubject": None,
    }
    job = collector._parse_history(_hist_json(ad))[0]
    assert "Owner" not in job
    assert "x509UserProxySubject" not in job


def test_jsonl_output_is_tolerated(collector):
    ad1 = {"GlobalJobId": "a#1.0#1", "ClusterId": 1, "ProcId": 0}
    ad2 = {"GlobalJobId": "b#2.0#2", "ClusterId": 2, "ProcId": 1}
    jsonl = (json.dumps(ad1) + "\n" + json.dumps(ad2)).encode("utf-8")
    jobs = collector._parse_history(jsonl)
    assert [j["ClusterId"] for j in jobs] == [1, 2]


def test_empty_output(collector):
    assert collector._parse_history(b"") == []
    assert collector._parse_history(b"  \n ") == []


def test_projection_expands_expressions_to_atoms(collector):
    proj = collector._projection_attributes()
    # bare keys present
    assert "CpusProvisioned" in proj and "Owner" in proj
    # expression atoms present, expression string itself absent
    assert "RemoteUserCpu" in proj and "RemoteSysCpu" in proj
    assert "RemoteUserCpu+RemoteSysCpu" not in proj
    # no duplicates, deterministic order (it is a list built from an ordered set)
    assert len(proj) == len(set(proj))


def test_is_valid_job_id():
    assert _is_valid_job_id((20096, 0))
    assert not _is_valid_job_id((20096, "/cms/Role=pilot/Capability=NULL"))
    assert not _is_valid_job_id((10800002, 14.5089285714286))
    assert not _is_valid_job_id((1,))
    assert not _is_valid_job_id(None)
    assert not _is_valid_job_id((True, 0))  # bools are not valid ids


class _FakeStateDB:
    def __init__(self, value):
        self._value = value
        self.set_calls = []

    def get(self, schedd, prefix):
        return self._value

    def set(self, schedd, prefix, cluster, proc):
        self.set_calls.append((cluster, proc))


def test_get_last_job_ignores_corrupt_checkpoint(collector):
    # Mirrors the production ce05 corruption: proc holds a string.
    collector.state_db = _FakeStateDB((20096, "/cms/Role=pilot/Capability=NULL"))
    assert collector.get_last_job("schedd@test") is None  # self-heals -> timestamp


def test_get_last_job_returns_valid_checkpoint(collector):
    collector.state_db = _FakeStateDB((20096, 0))
    assert collector.get_last_job("schedd@test") == (20096, 0)


def test_set_last_job_refuses_non_integer(collector):
    db = _FakeStateDB(None)
    collector.state_db = db
    collector.set_last_job("schedd@test", (10800002, 14.5089285714286))
    assert db.set_calls == []  # nothing persisted


def test_set_last_job_stores_valid(collector):
    db = _FakeStateDB(None)
    collector.state_db = db
    collector.set_last_job("schedd@test", (123, 4))
    assert db.set_calls == [(123, 4)]


def test_get_value_only_if_missing_key_returns_none():
    """only_if must not KeyError when its key is undefined (absent from the ad)."""
    from auditor_htcondor_collector.utils import get_value

    entry = {
        "name": "HEPSPEC",
        "key": "MachineAttrApelSpecs0",
        "matches": r"HEPSPEC\D+(\d+(\.\d+)?)",
        "only_if": {"key": "LastRemoteHost", "matches": r"^slot.+@execute$"},
    }
    job = {"MachineAttrApelSpecs0": "HEPSPEC 123"}  # LastRemoteHost absent
    assert get_value(entry, job) is None
