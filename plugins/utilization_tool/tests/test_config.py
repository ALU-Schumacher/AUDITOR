import tempfile

import pytest
import yaml

from auditor_utilization_plugin.config import (
    AuditorConfig,
    ClusterConfig,
    Config,
)

SAMPLE_YAML = {
    "logging": {"level": "INFO", "file": "app.log"},
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
    },
    "utilization": {
        "groupedby": "VOMS",
        "grouped_list": ["production", "lcgadmin", "ilc", "ops"],
        "watt_per_core": 4.6,
        "co2_per_kwh": 0.363,
        "interval": 10,
    },
    "cluster": {"watt_per_core": {"site": {"site1": 4.6, "site2": 4.3, "site3": 4.1}}},
    "email": {
        "enable_email_report": False,
        "smtp_server": "smtp.example.com",
        "smtp_port": 587,
    },
}


@pytest.fixture
def temp_yaml_file():
    with tempfile.NamedTemporaryFile("w+", delete=False, suffix=".yaml") as f:
        yaml.dump(SAMPLE_YAML, f)
        f.flush()
        yield f.name


def test_load_config_from_yaml(temp_yaml_file):
    cfg = Config.from_yaml(temp_yaml_file)
    assert cfg.logging.file == "app.log"
    assert cfg.logging.level == "INFO"
    assert cfg.auditor.hosts == ["localhost"]
    assert cfg.auditor.timeout == 60
    assert cfg.utilization.groupedby == "VOMS"
    assert cfg.cluster.sites == ["site1", "site2", "site3"]
    assert cfg.oneshot is False


def test_component_fields_loaded(temp_yaml_file):
    cfg = Config.from_yaml(temp_yaml_file)

    fields = cfg.auditor.component_fields_in_record

    assert fields.cores == "Cores"
    assert fields.benchmark == "HEPscore23"
    assert fields.total_cpu == "TotalCPU"


def test_tls_validation_requires_parameters():
    yaml_data = SAMPLE_YAML.copy()
    yaml_data["auditor"]["use_tls"] = True

    with pytest.raises(ValueError) as exc_info:
        AuditorConfig(**yaml_data["auditor"])
    assert (
        "Parameters ca_cert_path, client_cert_path, client_key_path are required"
        in str(exc_info.value)
    )


def test_tls_validation_passes_with_parameters():
    yaml_data = SAMPLE_YAML.copy()
    yaml_data["auditor"]["use_tls"] = True
    yaml_data["auditor"]["ca_cert_path"] = "/path/ca.pem"
    yaml_data["auditor"]["client_cert_path"] = "/path/client.pem"
    yaml_data["auditor"]["client_key_path"] = "/path/client-key.pem"

    cfg = AuditorConfig(**yaml_data["auditor"])
    assert cfg.use_tls is True
    assert cfg.ca_cert_path == "/path/ca.pem"


def test_cluster_sites_property():
    cluster_cfg = ClusterConfig(**SAMPLE_YAML["cluster"])
    sites = cluster_cfg.sites
    assert isinstance(sites, list)
    assert "site1" in sites and "site2" in sites and "site3" in sites


def test_default_oneshot():
    cfg = Config(**SAMPLE_YAML)
    assert cfg.oneshot is False
