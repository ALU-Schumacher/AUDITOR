#!/usr/bin/env python3

import logging
from asyncio import create_subprocess_exec, create_subprocess_shell
from asyncio.subprocess import PIPE
from datetime import datetime as dt
from datetime import timezone
from typing import List, Optional, Tuple

from pyauditor import (
    AuditorClient,
    AuditorClientBuilder,
    Component,
    Meta,
    Record,
    Score,
)

from .config import Config
from .exceptions import RecordGenerationException
from .state_db import StateDB
from .utils import get_value, maybe_convert


class CondorHistoryCollector(object):
    def __init__(self, config: Config):
        self.config = config
        self.logger = self.setup_logger()
        self.client = self.setup_auditor_client()
        self.state_db = StateDB(config.state_db)

    def setup_auditor_client(self) -> AuditorClient:
        """Sets up the auditor client."""
        builder = AuditorClientBuilder()
        addr = self.config.get("addr")
        port = self.config.get("port")
        if addr and port:
            self.logger.info(f"Using AUDITOR client at {addr}:{port}.")
            builder.address(addr, port)
        timeout = self.config.get("timeout")
        if timeout:
            self.logger.info(f"Using timeout of {timeout} seconds for AUDITOR client.")
            builder.timeout(timeout)
        tls_config = self.config.get("tls_config")
        if tls_config["use_tls"]:
            ca_cert_path = tls_config["ca_cert_path"]
            client_cert_path = tls_config["client_cert_path"]
            client_key_path = tls_config["client_key_path"]

            return builder.with_tls(
                client_cert_path, client_key_path, ca_cert_path
            ).build()
        else:
            return builder.build()

    def setup_logger(self) -> logging.Logger:
        """Sets up the logger for the collector."""
        logger = logging.getLogger("auditor.collectors.htcondor")
        logger.setLevel(self.config.log_level)
        if self.config.log_file:
            from logging.handlers import RotatingFileHandler

            handler = RotatingFileHandler(
                self.config.log_file,
                maxBytes=10 * 1024 * 1024,
                backupCount=5,
            )
        else:
            handler = logging.StreamHandler()
        handler.setFormatter(
            logging.Formatter(
                "{asctime} - {name} - {levelname: <8} - {message}", style="{"
            )
        )
        logger.addHandler(handler)
        return logger

    async def run(self):
        self.logger.info("Starting collector run.")
        with self.state_db.connection():
            for schedd_name in self.config.schedd_names:
                id = self.config.get("job_id") or ""
                await self._collect(schedd_name, job_id=id)
        self.logger.info("Collector run finished.")

    async def _collect(self, schedd_name: str, job_id: str = ""):
        """Collects jobs from `condor_history` for a given schedd
        and adds them to the auditor client.

        Jobs are iterated over in reverse order,
        so that the oldest job is processed first.
        """
        self.logger.info(f"Collecting jobs for schedd {schedd_name!r}.")
        # Convert Job ID to (cluster, proc) tuple
        parsed_job_id: Optional[Tuple[int, int]] = None
        if job_id:
            try:
                if "." in job_id:
                    parsed_job_id = tuple(map(int, job_id.split(".")))
                else:
                    parsed_job_id = (int(job_id), 0)
            except ValueError:
                raise ValueError("Invalid job id.")
        else:
            parsed_job_id = self.get_last_job(schedd_name)

        self.logger.debug(f"Using job id {parsed_job_id}.")

        jobs = await self.query_htcondor_history(schedd_name, parsed_job_id)

        added, failed = 0, 0
        for job in reversed(jobs):
            try:
                record = self._generate_record(job)
                await self.client.add(record)
                added += 1
            except RecordGenerationException as e:
                failed += 1
                self.logger.debug(e.args[0])
            self.set_last_job(schedd_name, (job["ClusterId"], job["ProcId"]))
        self.logger.info(
            f"Added {added} records."
            f"{f' Failed to generate {failed} records.' if failed else ''}"
        )

    def get_last_job(self, schedd_name: str) -> Optional[Tuple[int, int]]:
        """Returns the last job id that was processed for a given schedd and prefix."""
        job = self.state_db.get(schedd_name, self.config.record_prefix)
        if job is None:
            self.logger.warning(
                f"Could not find last job id for schedd {schedd_name!r} and record "
                f"prefix {self.config.record_prefix!r}. Starting from timestamp."
            )
            return None
        return job

    def set_last_job(self, schedd_name: str, job_id: Tuple[int, int]):
        """Sets the last job id that was processed for a given schedd and prefix."""
        self.state_db.set(schedd_name, self.config.record_prefix, *job_id)
        self.logger.debug(f"Set last job id to {job_id} for schedd {schedd_name!r}.")

    async def query_htcondor_history(
        self, schedd_name: str, job: Optional[Tuple[int, int]]
    ) -> List[dict]:
        """Queries HTCondor history for jobs with a given schedd name and job id."""
        if job is not None:
            assert type(job) is tuple and len(job) == 2, "Invalid job id"
            assert isinstance(job[0], int) and isinstance(job[1], int), "Invalid job id"

        def escape(arg: str) -> str:
            """Escape a CLI argument to avoid interpretation for the chosen execution type"""
            if self.config.query_type == "shell":
                return f"'{arg}'"
            elif self.config.query_type == "exec":
                return arg
            else:
                raise NotImplementedError(f"query_type {self.config.query_type!r}")

        if job is None:
            self.logger.debug(
                f"Querying HTCondor history for {schedd_name!r} starting "
                f"from {dt.fromtimestamp(self.config.condor_timestamp)}."
            )
            # need to exclude 0 CompletionDate, see AUDITOR#1309
            since = f"CompletionDate <= {self.config.condor_timestamp} && CompletionDate > 0"
        else:
            self.logger.debug(
                f"Querying HTCondor history for {schedd_name!r} "
                f"starting from job {job}."
            )
            since = f"{job[0]}.{job[1]}"

        cmd: list[str] = [
            "condor_history",
            "-backwards",
            "-wide",
            "-name",
            schedd_name,
            "-since",
            escape(since),
            "-af:,",
            *self.config.class_ads,
        ]
        if self.config.get("pool"):
            cmd.extend(["-pool", self.config.pool])
        # multiple `-constraint`s are implicitly &&'ed by HTCondor
        if self.config.get("job_status"):
            job_stats = " || ".join(f"JobStatus == {j}" for j in self.config.job_status)
            cmd.extend(["-constraint", escape(job_stats)])
        if constraint := self.config.get("constraint"):
            cmd.extend(["-constraint", constraint])
        if self.config.get("history_file"):
            cmd.extend(["-file", escape(self.config.history_file)])

        if self.config.query_type == "shell":
            self.logger.debug(f"Running command: {' '.join(cmd)!r}")
            p = await create_subprocess_shell(" ".join(cmd), stdout=PIPE, stderr=PIPE)
        elif self.config.query_type == "exec":
            self.logger.debug(f"Running command: {cmd!r}")
            p = await create_subprocess_exec(*cmd, stdout=PIPE, stderr=PIPE)
        else:
            raise NotImplementedError(f"query_type {self.config.query_type!r}")
        output, err = await p.communicate()
        if err:
            self.logger.error(f"Error querying HTCondor history:\n{err}")

        jobs = [
            map(maybe_convert, map(str.strip, job.decode("utf-8").split(",")))
            for job in output.strip().splitlines()
        ]

        return [dict(zip(self.config.class_ads, job)) for job in jobs]

    def _generate_components(self, job: dict) -> List[Component]:
        components = []
        for component in self.config.components:
            amount = get_value(component, job)
            self.logger.debug(
                f"Got amount {amount!r} ({type(amount)}) for component {component!r}."
            )
            if amount is not None:
                try:
                    # AUDITOR expects int-values for components
                    amount = int(amount)
                except ValueError:
                    self.logger.warning(
                        f"Could not convert amount ({amount!r}) for component "
                        f"{component['name']!r} of job {job['GlobalJobId']!r} to int. "
                        f"Skipping record."
                    )
                    raise ValueError
                comp = Component(name=component["name"], amount=amount)
                for score in component.get("scores", []):
                    value = get_value(score, job) or "0.0"
                    comp.with_score(Score(score["name"], float(value)))
                components.append(comp)
            else:
                self.logger.warning(
                    f"Could not get value for component {component['name']!r} "
                    f"({component!r}) for job {job['GlobalJobId']!r}."
                )
                raise ValueError
        return components

    def _get_meta(self, job: dict) -> Meta:
        meta = Meta()
        for key, entry in self.config.meta.items():
            values = []
            for item in entry if isinstance(entry, list) else [entry]:
                value = get_value(item, job)
                if value is not None:
                    values.append(str(value))
                    if key == "site":  # site is a special case
                        break
            if values:
                meta.insert(key, values)
            else:
                self.logger.warning(
                    f"Could not find meta value for {key!r} "
                    f"for job {job['GlobalJobId']!r}."
                )
        return meta

    def _generate_record(self, job: dict) -> Record:
        job_id = job["GlobalJobId"]

        self.logger.debug(f"Generating record for job {job_id!r}.")

        # Get the start time of the job
        start_time = None
        for key in ["LastMatchTime"]:
            if key in job and job[key] != "undefined":
                start_time = job[key]
                break
        if start_time is None:
            raise RecordGenerationException(job_id, "Could not find start time.")

        # Get the stop time of the job
        stop_time = None
        for key in ["CompletionDate", "EnteredCurrentStatus"]:
            if key in job and job[key] != "undefined":
                stop_time = job[key]
                break
        if stop_time is None:
            self.logger.debug(f"Could not find stop time for job {job_id!r}.")
            stop_time = 0

        meta = self._get_meta(job)

        try:
            record_id = f"{self.config.record_prefix}-{job_id}"
            record = Record(
                record_id=record_id,
                start_time=dt.fromtimestamp(start_time, tz=timezone.utc),
            )
            record.with_stop_time(
                dt.fromtimestamp(stop_time, tz=timezone.utc)
            ).with_meta(meta)
            for component in self._generate_components(job):
                record.with_component(component)
        except (KeyError, ValueError) as e:
            self.logger.error(f"Error generating record for job {job_id!r}:\n{e}")
            raise RecordGenerationException(job_id)

        self.logger.debug(
            f"Generated record for job {job_id!r} under DB record key {record_id}."
        )
        return record
