from unittest.mock import MagicMock, patch

import pytest
import yaml

from auditor_utilization_plugin.config import AuditorConfig, ComponentFieldsConfig
from auditor_utilization_plugin.main import (
    build_auditor_client,
    iter_endpoints,
    load_config,
    override_config,
)

SAMPLE_CONFIG = {
    "auditor": {
        "hosts": ["localhost"],
        "port": [8001],
        "timeout": 60,
        "site_meta_field": "site_id",
        "use_tls": False,
        "component_fields_in_record": {
            "cores": "Cores",
            "benchmark": "HEPscore23",
            "total_cpu": "TotalCPU",
        },
    }
}


def test_load_config(tmp_path):
    file = tmp_path / "config.yaml"
    file.write_text(yaml.dump(SAMPLE_CONFIG))

    data = load_config(file)

    assert "auditor" in data
    assert data["auditor"]["hosts"] == ["localhost"]


def test_override_config():
    config = {
        "auditor": {
            "hosts": ["oldhost"],
            "port": [8000],
            "timeout": 10,
        },
        "utilization": {"interval": 5},
    }

    args = MagicMock()
    args.host = "newhost"
    args.port = 9000
    args.timeout = 30
    args.interval = 20

    updated = override_config(config, args)

    assert updated["auditor"]["hosts"] == ["newhost"]
    assert updated["auditor"]["port"] == [9000]
    assert updated["auditor"]["timeout"] == 30
    assert updated["utilization"]["interval"] == 20


def test_iter_endpoints_success():
    auditor_cfg = AuditorConfig(
        hosts=["h1", "h2"],
        port=[8001, 8002],
        timeout=60,
        site_meta_field="site_id",
        use_tls=False,
        component_fields_in_record=ComponentFieldsConfig(
            cores="Cores", benchmark="HEPscore23", total_cpu="TotalCPU"
        ),
    )

    endpoints = iter_endpoints(auditor_cfg)

    assert endpoints == [("h1", 8001), ("h2", 8002)]


def test_iter_endpoints_mismatch():
    auditor_cfg = AuditorConfig(
        hosts=["h1"],
        port=[8001, 8002],
        timeout=60,
        site_meta_field="site_id",
        use_tls=False,
        component_fields_in_record=ComponentFieldsConfig(
            cores="Cores", benchmark="HEPscore23", total_cpu="TotalCPU"
        ),
    )

    with pytest.raises(ValueError):
        iter_endpoints(auditor_cfg)


@patch("auditor_utilization_plugin.main.AuditorClientBuilder")
def test_build_auditor_client(mock_builder):
    mock_instance = MagicMock()
    mock_builder.return_value = mock_instance
    mock_instance.address.return_value = mock_instance
    mock_instance.timeout.return_value = mock_instance
    mock_instance.build_blocking.return_value = "client"

    auditor_cfg = AuditorConfig(
        hosts=["localhost"],
        port=[8001],
        timeout=60,
        site_meta_field="site_id",
        use_tls=False,
        component_fields_in_record=ComponentFieldsConfig(
            cores="Cores", benchmark="HEPscore23", total_cpu="TotalCPU"
        ),
    )

    client = build_auditor_client(auditor_cfg, "localhost", 8001)

    assert client == "client"
    mock_instance.address.assert_called_once_with("localhost", 8001)
