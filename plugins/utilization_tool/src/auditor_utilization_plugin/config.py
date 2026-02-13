import logging
from typing import Dict, List, Optional, Union

import yaml
from pydantic import BaseModel, Field, model_validator

logger = logging.getLogger("utilization")


class LoggingConfig(BaseModel):
    level: str = Field(..., description="Logging level, e.g. INFO or DEBUG")
    file: str = Field(..., description="Path to log file")


class EmailServerConfig(BaseModel):
    enable_email_report: bool = Field(..., description="Enable or disable email report")
    smtp_server: str = Field(..., description="SMTP server")
    smtp_port: int = Field(..., description="SMTP port")


class AuditorConfig(BaseModel):
    hosts: List[str] = Field(..., description="List of auditor host machines")
    port: List[int] = Field(
        ..., description="List of ports corresponding to each AUDITOR host"
    )
    timeout: int = Field(
        ..., description="timeout (in seconds) for requests sent to AUDITOR"
    )
    site_meta_field: Union[str, List[str]] = Field(
        ...,
        description="Site meta fields to filter sites (can be a string or a list of strings [site_id, site])",
    )
    use_tls: bool = Field(
        ..., description="Enable TLS for connection to the AUDITOR service"
    )
    ca_cert_path: Optional[str] = Field(
        None, description="Path to the CA certificate file"
    )
    client_cert_path: Optional[str] = Field(
        None,
        description="Path to the client certificate file for mutual TLS authentication with the AUDITOR service",
    )
    client_key_path: Optional[str] = Field(
        None,
        description="Path to the client private key file corresponding to the client certificate for mutual TLS",
    )

    @model_validator(mode="after")
    def check_tls_config(self):
        if self.use_tls:
            missing_parameters = [
                parameter_name
                for parameter_name, value in {
                    "ca_cert_path": self.ca_cert_path,
                    "client_cert_path": self.client_cert_path,
                    "client_key_path": self.client_key_path,
                }.items()
                if value is None
            ]
            if missing_parameters:
                missing_parameters_str = ", ".join(missing_parameters)
                raise ValueError(
                    f"Parameters {missing_parameters_str} are required if use_tls: True"
                )
        return self


class UtilisationConfig(BaseModel):
    groupedby: str = Field(..., description="Field to group utilisation data by")
    grouped_list: List[str] = Field(..., description="List of groups to include")
    watt_per_core: float = Field(..., description="Watts per CPU core")
    co2_per_kwh: float = Field(..., description="CO2 emitted per kWh (kg)")
    interval: int = Field(..., description="Reporting interval in seconds")
    file_name: Optional[str] = Field(
        default="auditor",
        description="Prefix of filename that stores the summary by month",
    )
    file_path: Optional[str] = Field(
        default=".", description="File path to store the summary CSV"
    )


class ClusterConfig(BaseModel):
    watt_per_core: Dict[str, Dict[str, float]] = Field(
        ..., description="Nested mapping of site → watt_per_core values"
    )

    @property
    def sites(self) -> List[str]:
        return list(self.watt_per_core.get("site", {}).keys())


class Config(BaseModel):
    logging: LoggingConfig
    auditor: AuditorConfig
    utilisation: UtilisationConfig
    cluster: ClusterConfig
    oneshot: bool = False
    email: EmailServerConfig

    @classmethod
    def from_yaml(cls, path: str) -> "Config":
        """Load configuration from a YAML file."""
        with open(path, "r") as f:
            data = yaml.safe_load(f)
        return cls(**data)


if __name__ == "__main__":
    cfg = Config.from_yaml("config.yaml")

    print(f"Logging to: {cfg.logging.file}")
    print(f"Auditor hosts: {cfg.auditor.hosts}")
    print(f"Sites: {cfg.cluster.sites}")
    print(f"Utilisation groups: {cfg.utilisation.grouped_list}")
