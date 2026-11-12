import argparse
import datetime
import json
from copy import deepcopy
from unittest.mock import AsyncMock, MagicMock, patch
from urllib.parse import parse_qs

import pandas as pd
import pytest
from auditor_utilization_plugin.config import CollectorType, ComponentFieldsConfig
from auditor_utilization_plugin.utilization import (
    build_query,
    categorize_power,
    generate_utilization_report,
    get_stats_by_user,
    map_user_name,
    record_to_dict,
    records_to_df,
    rename_user,
)


@pytest.mark.parametrize(
    "filters",
    [
        None,
        {},
        {"site": "RHUL-LOCAL"},
        {"group": "LOCAL"},
        {"site": "RHUL-LOCAL", "group": "LOCAL"},
        {"GROUP": "LOCAL"},
        {"site": "a & b/+"},
    ],
)
def test_build_query_preserves_time_bounds_and_metadata(filters):
    start = datetime.datetime(2026, 10, 1, tzinfo=datetime.timezone.utc)
    end = datetime.datetime(2026, 10, 2, tzinfo=datetime.timezone.utc)
    original = parse_qs(build_query(start, end))
    query = parse_qs(build_query(start, end, filters))
    assert query["stop_time[gte]"] == original["stop_time[gte]"]
    assert query["stop_time[lt]"] == original["stop_time[lt]"]
    expected = {**original}
    for key, value in (filters or {}).items():
        expected[f"meta[{key}][c][0]"] = [value]
    assert query == expected


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


@pytest.fixture
def config_auditor_collector_type():
    return CollectorType("htcondor")


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "filters, expected_users",
    [
        (
            {},
            {"local-user", "other-user", "grid-user", "shared-user", "unlabelled-user"},
        ),
        ({"site": "RHUL-LOCAL"}, {"local-user", "other-user"}),
        ({"group": "LOCAL"}, {"local-user", "shared-user"}),
        ({"site": "RHUL-LOCAL", "group": "LOCAL"}, {"local-user"}),
        ({"site": "unknown"}, set()),
        ({"site": "rhul-local"}, set()),
    ],
)
async def test_report_filters_are_independent_of_user_grouping(
    sample_json_record, config_component_values, filters, expected_users
):
    stored_records = []
    for user, site, group in [
        ("local-user", "RHUL-LOCAL", "LOCAL"),
        ("other-user", "RHUL-LOCAL", "ATLAS"),
        ("grid-user", "RHUL", "ATLAS"),
        ("shared-user", "RHUL", "LOCAL"),
    ]:
        record = deepcopy(sample_json_record)
        record["meta"].update(user=[user], site=[site], group=[group])
        if user == "local-user":
            record["meta"]["site"].insert(0, "another-site")
        stored_records.append(record)

    unlabelled = deepcopy(sample_json_record)
    unlabelled["meta"]["user"] = ["unlabelled-user"]
    stored_records.append(unlabelled)

    def query_records(query):
        params = parse_qs(query)
        for key, value in filters.items():
            assert params[f"meta[{key}][c][0]"] == [value]
        if not params["stop_time[gte]"][0].startswith("2025-07-16"):
            return []
        result = []
        for record in stored_records:
            if all(
                values[0] in record["meta"].get(key[5:].split("]")[0], [])
                for key, values in params.items()
                if key.startswith("meta[")
            ):
                item = MagicMock()
                item.to_json.return_value = json.dumps(record)
                result.append(item)
        return result

    client = MagicMock()
    client.advanced_query.side_effect = query_records
    config = MagicMock()
    config.oneshot = True
    config.cluster.watt_per_core = {}
    config.utilization.watt_per_core = 4.6
    config.utilization.groupedby = "user"
    config.utilization.grouped_list = []
    config.utilization.metadata_filters = filters
    config.utilization.co2_per_kwh = 0.363
    config.auditor.component_fields_in_record = config_component_values
    config.auditor.collector_type = CollectorType.htcondor
    config.email.enable_email_report = True
    args = argparse.Namespace(month=7, year=2025, oneshot=True)

    with (
        patch(
            "auditor_utilization_plugin.utilization.mailing_list",
            new_callable=AsyncMock,
        ) as sender,
        patch("auditor_utilization_plugin.utilization.write_to_csv") as writer,
        pytest.raises(SystemExit),
    ):
        await generate_utilization_report(
            MagicMock(), config, args, client, "localhost"
        )

    assert client.advanced_query.call_count == 31
    if expected_users:
        sender.assert_awaited_once()
        writer.assert_called_once()
        assert set(writer.call_args.args[0]["user"]) == expected_users
        assert set(sender.call_args.args[2]["user"]) == expected_users
    else:
        sender.assert_not_awaited()
        writer.assert_not_called()


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
        collector_type=config_auditor_collector_type,
    )

    # Only one user in this sample
    assert len(data["user"]) == 1

    # CPU efficiency = TotalCPU / (runtime * Cores)
    total_cpu = df["TotalCPU"].sum()
    total_core_time = (df["runtime"] * df["Cores"]).sum()
    expected_cpu_eff = total_cpu / total_core_time
    assert data["cpu_eff"][0] == pytest.approx(expected_cpu_eff)

    # corehours = sum(Cores * runtime) / 3600  (for htcondor/kubernetes, we don't divide by 1000 since the cputime is already in seconds)
    expected_corehours = (df["Cores"] * df["runtime"]).sum() / 3600.0
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
