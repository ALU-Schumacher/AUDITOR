import json
from unittest.mock import MagicMock

import pandas as pd
import pytest

from auditor_utilization_plugin.config import ComponentFieldsConfig
from auditor_utilization_plugin.utilization import (
    categorize_power,
    get_stats_by_user,
    map_user_name,
    record_to_dict,
    records_to_df,
    rename_user,
)


@pytest.fixture
def sample_json_record():
    sample_json = """{
    "record_id":"record-1",
    "meta":{"VOMS":["/atlas/Role=production/Capability=NULL"],"user":["user-1"]},
    "components":[
        {"name":"Cores","amount":1,"scores":[{"name":"HEPscore23","value":18.4}]},
        {"name":"TotalCPU","amount":72327,"scores":[]}
           ],
    "start_time":"2025-07-15T20:39:48Z",
    "stop_time":"2025-07-16T13:17:47Z",
    "runtime":59879
    }"""
    return json.loads(sample_json)


@pytest.fixture
def sample_df(sample_json_record):
    return pd.DataFrame([record_to_dict(sample_json_record)])


@pytest.fixture
def config_values():
    return {
        "co2_per_kwh": 0.363,
        "grouped_list": ["production", "test"],
        "watt_per_core_default": 4.6,
        "watt_per_core_site": {"bfg": 4.6, "site-2": 4.3, "site-3": 4.1},
    }


@pytest.fixture
def config_component_values():
    return ComponentFieldsConfig(
        cores="Cores", benchmark="HEPscore23", total_cpu="TotalCPU"
    )


def test_record_to_dict(sample_json_record):
    d = record_to_dict(sample_json_record)
    assert d["record_id"].startswith("record")
    assert d["Cores"] == 1
    assert d["HEPscore23"] == 18.4
    assert d["TotalCPU"] == 72327
    assert d["runtime"] == 59879


def test_records_to_df(sample_json_record):
    mock_record = MagicMock()
    mock_record.to_json.return_value = json.dumps(sample_json_record)
    df = records_to_df([mock_record])
    assert isinstance(df, pd.DataFrame)
    assert "Cores" in df.columns
    assert df.loc[0, "HEPscore23"] == 18.4


def test_rename_user_realistic():
    vo = "/atlas/Role=production/Capability=NULL"
    name = rename_user(vo)
    assert name == "atlas-production"


def test_map_user_name(sample_df, config_values):
    df = map_user_name(sample_df, "VOMS", config_values["grouped_list"])
    assert "names" in df.columns
    assert df.loc[0, "names"] == "production"


def test_categorize_power(config_values):
    d = config_values["watt_per_core_site"]
    assert categorize_power("bfg", d) == 4.6
    assert categorize_power("site-2", d) == 4.3
    assert categorize_power("unknown", d) is None


def test_get_stats_by_user_with_config(
    sample_df, config_values, config_component_values
):
    df = sample_df.copy()
    df["watt_per_core"] = config_values["watt_per_core_default"]

    data = get_stats_by_user(
        df,
        config_values["co2_per_kwh"],
        grouped="VOMS",
        grouped_list=config_values["grouped_list"],
        component_fields_in_record=config_component_values,
    )

    # Only one user in this sample
    assert len(data["user"]) == 1

    # CPU efficiency = TotalCPU / (runtime * Cores)
    total_cpu = df["TotalCPU"].sum()
    total_core_time = (df["runtime"] * df["Cores"]).sum()
    expected_cpu_eff = total_cpu / total_core_time
    assert data["cpu_eff"][0] == pytest.approx(expected_cpu_eff)

    # corehours = sum(Cores * runtime) / 3600 / 1000
    expected_corehours = (df["Cores"] * df["runtime"]).sum() / 3600.0 / 1000.0
    assert data["corehours"][0] == pytest.approx(expected_corehours)

    # wall_work = sum(HEPscore23 * Cores * runtime) / 3600 / 1000
    expected_khs23h = (
        (df["HEPscore23"] * df["Cores"] * df["runtime"]).sum() / 3600.0 / 1000.0
    )
    assert data["khs23h"][0] == pytest.approx(expected_khs23h)

    # power [kWh] = sum(watt_per_core * Cores * runtime) / 3600 / 1000
    expected_power = (
        (df["watt_per_core"] * df["Cores"] * df["runtime"]).sum() / 3600.0 / 1000.0
    )
    assert data["power [kWh]"][0] == pytest.approx(expected_power)

    # CO2 = power * co2_per_kwh
    expected_co2 = expected_power * config_values["co2_per_kwh"]
    assert data["co2 [kg]"][0] == pytest.approx(expected_co2)
