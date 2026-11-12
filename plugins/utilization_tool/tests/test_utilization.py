import argparse
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pandas as pd
import pytest
from auditor_utilization_plugin.config import CollectorType, ComponentFieldsConfig
from auditor_utilization_plugin.utilization import (
    categorize_power,
    generate_utilization_report,
    get_stats_by_user,
    mailing_list,
    map_user_name,
    record_to_dict,
    records_to_df,
    rename_user,
)


@pytest.mark.asyncio
async def test_mailing_list_passes_report_period():
    df = pd.DataFrame({"user": ["test"]})
    logger = MagicMock()
    with patch(
        "auditor_utilization_plugin.utilization.send_email", new_callable=AsyncMock
    ) as sender:
        await mailing_list(
            "smtp.test.com", 587, df, logger, report_period="December 2025"
        )
        sender.assert_awaited_once_with(
            "smtp.test.com", 587, df, logger, report_period="December 2025"
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


@pytest.fixture
def config_auditor_collector_type():
    return CollectorType("htcondor")


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


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "month, year, expected_period",
    [(12, 2025, "December 2025"), (1, 2026, "January 2026")],
)
async def test_report_email_uses_requested_period(
    sample_json_record, config_component_values, month, year, expected_period
):
    record = MagicMock()
    record.to_json.return_value = json.dumps(sample_json_record)
    client = MagicMock()
    client.advanced_query.side_effect = [[record]] + [[]] * 31
    config = MagicMock()
    config.oneshot = True
    config.cluster.watt_per_core = {}
    config.utilization.watt_per_core = 4.6
    config.utilization.groupedby = "VOMS"
    config.utilization.grouped_list = ["production"]
    config.utilization.co2_per_kwh = 0.363
    config.auditor.component_fields_in_record = config_component_values
    config.auditor.collector_type = CollectorType.htcondor
    config.email.enable_email_report = True
    config.email.smtp_server = "smtp.test.com"
    config.email.smtp_port = 25
    args = argparse.Namespace(month=month, year=year, oneshot=True)

    with (
        patch(
            "auditor_utilization_plugin.utilization.mailing_list",
            new_callable=AsyncMock,
        ) as sender,
        patch("auditor_utilization_plugin.utilization.write_to_csv"),
        pytest.raises(SystemExit),
    ):
        await generate_utilization_report(
            MagicMock(), config, args, client, "localhost"
        )

    sender.assert_awaited_once()
    assert sender.call_args.kwargs["report_period"] == expected_period
