#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2026 Raghuvar Vijayakumar <raghuvarvijayakumar@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import argparse
import asyncio
import logging
import signal
import sys
from logging import Logger
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Any, Dict, List, Tuple

import yaml
from pyauditor import AuditorClientBuilder

from auditor_utilization_plugin.config import AuditorConfig, Config
from auditor_utilization_plugin.utilization import generate_utilization_report

TRACE = logging.DEBUG - 5


def load_config(path: Path) -> Dict[str, Any]:
    path = Path(path)
    try:
        with path.open("r", encoding="utf-8") as file:
            config = yaml.safe_load(file) or {}
        logging.info(f"Loaded configuration from {path}")
        return config
    except yaml.YAMLError as e:
        logging.info(f"YAML parsing error in {path}: {e}")
        raise ValueError(f"Invalid YAML format in {path}") from e
    except PermissionError:
        logging.info(f"Permission denied reading {path}")
        raise
    except Exception as e:
        logging.info(f"Unexpected error loading configuration from {path}: {e}")
        raise


def override_config(config, args):
    if args.host:
        config["auditor"]["hosts"] = [args.host]
    if args.port:
        config["auditor"]["port"] = [args.port]
    if args.timeout:
        config["auditor"]["timeout"] = args.timeout
    if args.interval:
        config["utilization"]["interval"] = args.interval
    return config


def setup_logging(config: Config) -> Logger:
    """Set up global logging."""
    log_level = getattr(logging, config.logging.level.upper(), logging.INFO)
    log_file = config.logging.file

    log_format = "[%(asctime)s] %(levelname)-8s %(message)s (%(filename)s:%(lineno)d)"
    date_format = "%Y-%m-%d %H:%M:%S"

    logging.addLevelName(TRACE, "TRACE")
    logging.basicConfig(level=log_level, format=log_format, datefmt=date_format)

    logging.basicConfig(
        level=log_level,
        format=log_format,
        datefmt=date_format,
        force=True,
    )

    logger = logging.getLogger("utilization")

    if log_file:
        handler = RotatingFileHandler(
            log_file, maxBytes=10 * 1024 * 1024, backupCount=5
        )
        handler.setFormatter(logging.Formatter(log_format, date_format))
        logger.addHandler(handler)

    return logger


def iter_endpoints(auditor_cfg: AuditorConfig) -> List[Tuple[str, int]]:
    hosts = list(auditor_cfg.hosts)
    ports = list(auditor_cfg.port)

    if len(hosts) != len(ports):
        raise ValueError(
            f"auditor.hosts ({len(hosts)}) and auditor.port ({len(ports)}) must match length"
        )

    return list(zip(hosts, ports))


def build_auditor_client(
    auditor_cfg: AuditorConfig, host: str, port: int
) -> AuditorClientBuilder:
    """Create and configure the Auditor client."""
    builder = AuditorClientBuilder()

    timeout = auditor_cfg.timeout

    if getattr(auditor_cfg, "use_tls", False):
        builder = builder.with_tls(
            auditor_cfg.client_cert_path,
            auditor_cfg.client_key_path,
            auditor_cfg.ca_cert_path,
        )

    builder = builder.address(host, port).timeout(timeout)
    client = builder.build_blocking()

    return client


async def run_one_endpoint(
    logger: Logger, config: Config, args: argparse.Namespace, host: str, port: int
) -> None:
    client = build_auditor_client(config.auditor, host, port)

    logger.info(f"Connected to auditor {host} {port}")
    await generate_utilization_report(logger, config, args, client, host)


async def main():
    parser = argparse.ArgumentParser(
        prog="EGI_validation",
        description="creates monthly summary to compare with EGI values",
        epilog="""
Example:
  auditor-utilization-plugin -c config.yaml --month 9 --year 2025 --oneshot

Outputs a CSV summary
""",
    )

    parser.add_argument("--port", type=int, dest="port", help="Port", default=None)
    parser.add_argument("--host", type=str, dest="host", help="hostname", default=None)
    parser.add_argument(
        "--interval", type=int, dest="interval", help="Report last n days", default=7
    )
    parser.add_argument(
        "--month", type=int, dest="month", help="Report specific month", default=None
    )
    parser.add_argument(
        "--year", type=int, dest="year", help="Report specific year", default=None
    )

    parser.add_argument(
        "--timeout",
        type=int,
        dest="timeout",
        help="Increase timeout if needed",
        default=60,
    )
    parser.add_argument("--save_data", action="store_true")

    parser.add_argument("--oneshot", action="store_true", help="Run one-shot")

    parser.add_argument(
        "-c", "--config", required=True, help="Path to YAML configuration file"
    )
    args = parser.parse_args()
    config = load_config(args.config)
    config = override_config(config, args)

    config = Config.from_yaml(args.config)

    logger = setup_logging(config)
    logger.info("Starting utilization plugin")

    endpoints = iter_endpoints(config.auditor)

    tasks = []

    for host, port in endpoints:
        task = run_one_endpoint(logger, config, args, host, port)
        tasks.append(task)

    await asyncio.gather(*tasks)

    logger.critical("Utilization plugin stopped")


def shutdown():
    logging.info("\nExiting gracefully...")
    sys.exit(0)


signal.signal(signal.SIGINT, lambda sig, frame: shutdown())


def cli():
    asyncio.run(main())


if __name__ == "__main__":
    cli()
