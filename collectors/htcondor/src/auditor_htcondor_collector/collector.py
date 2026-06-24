#!/usr/bin/env python3

import json
import logging
import re
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
from .utils import get_value

# A bare ClassAd attribute name (used to tell attribute keys from expressions).
_BARE_ATTR_RE = re.compile(r"\A[A-Za-z_][A-Za-z0-9_]*\Z")
# Any attribute name occurring inside an expression.
_ATTR_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
# A sum/difference of attributes and numeric literals, e.g. "A+B-3".
_SUM_EXPR_RE = re.compile(r"\A[A-Za-z0-9_.]+(?:[+-][A-Za-z0-9_.]+)*\Z")


def _is_valid_job_id(job_id) -> bool:
    """True iff `job_id` is a (cluster, proc) pair of plain ints (not bools)."""
    return (
        isinstance(job_id, tuple)
        and len(job_id) == 2
        and all(isinstance(v, int) and not isinstance(v, bool) for v in job_id)
    )


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
            self.set_last_job(
                schedd_name, (job.get("ClusterId"), job.get("ProcId"))
            )
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
        if not _is_valid_job_id(job):
            # A previously stored checkpoint may be corrupt (e.g. written by an
            # older version that mis-parsed `condor_history` output). Do not let
            # it crash the collector; ignore it and resume from the timestamp.
            self.logger.warning(
                f"Stored checkpoint {job!r} for schedd {schedd_name!r} and record "
                f"prefix {self.config.record_prefix!r} is not a valid (int, int) "
                f"job id. Ignoring it and starting from timestamp."
            )
            return None
        return job

    def set_last_job(self, schedd_name: str, job_id: Tuple[int, int]):
        """Sets the last job id that was processed for a given schedd and prefix."""
        if not _is_valid_job_id(job_id):
            # Never persist a non-integer job id: that would crash the next run
            # on read. Leave the previous checkpoint in place instead.
            self.logger.warning(
                f"Refusing to store invalid job id {job_id!r} for schedd "
                f"{schedd_name!r}; keeping the previous checkpoint."
            )
            return
        self.state_db.set(schedd_name, self.config.record_prefix, *job_id)
        self.logger.debug(f"Set last job id to {job_id} for schedd {schedd_name!r}.")

    async def query_htcondor_history(
        self, schedd_name: str, job: Optional[Tuple[int, int]]
    ) -> List[dict]:
        """Queries HTCondor history for jobs with a given schedd name and job id."""
        if job is not None and not _is_valid_job_id(job):
            # Be defensive rather than fatal: a malformed job id must not abort
            # the whole collector run. Fall back to the timestamp-based query.
            self.logger.warning(
                f"Ignoring invalid job id {job!r}; starting from timestamp."
            )
            job = None

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
            "-json",
            "-attributes",
            escape(",".join(self._projection_attributes())),
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

        return self._parse_history(output)

    def _projection_attributes(self) -> List[str]:
        """Plain ClassAd attribute names to request from `condor_history`.

        `condor_history` only projects attribute *names* (not expressions) in
        `-json` mode, so for expression keys such as
        ``RemoteUserCpu+RemoteSysCpu`` the referenced attribute names are
        requested and the expression is evaluated locally (see
        `_eval_expression`).
        """
        attrs: "dict[str, None]" = {}
        for class_ad in self.config.class_ads:
            if _BARE_ATTR_RE.match(class_ad):
                attrs[class_ad] = None
            else:
                for name in _ATTR_RE.findall(class_ad):
                    attrs[name] = None
        return list(attrs)

    def _parse_history(self, output: bytes) -> List[dict]:
        """Parse `condor_history -json` output into a list of job dicts.

        Parsing structured JSON instead of comma-separated autoformat avoids
        column misalignment when a ClassAd value contains a comma (e.g. an
        X.509 subject with ``O=Fermi Forward Discovery Group, LLC``). JSON also
        yields correctly typed values, so ``ClusterId``/``ProcId`` are real
        ints. Undefined/null attributes are dropped so downstream lookups see
        them as missing, and expression keys are evaluated locally.
        """
        text = output.decode("utf-8").strip()
        if not text:
            return []
        try:
            ads = json.loads(text)
        except json.JSONDecodeError:
            # Tolerate JSON-lines output (`-jsonl`): one JSON object per line.
            ads = []
            for line in text.splitlines():
                line = line.strip().rstrip(",")
                if not line or line in ("[", "]"):
                    continue
                try:
                    ads.append(json.loads(line))
                except json.JSONDecodeError as e:
                    self.logger.warning(f"Skipping unparseable history line: {e}")

        jobs = []
        for ad in ads:
            job = {
                key: value
                for key, value in ad.items()
                if value is not None and value != "undefined"
            }
            self._add_expression_values(job)
            jobs.append(job)
        return jobs

    def _add_expression_values(self, job: dict) -> None:
        """Populate expression-valued class_ads (e.g. CPU-time sums) in-place."""
        for class_ad in self.config.class_ads:
            if class_ad in job or _BARE_ATTR_RE.match(class_ad):
                continue
            value = self._eval_expression(class_ad, job)
            if value is not None:
                job[class_ad] = value

    def _eval_expression(self, expr: str, job: dict):
        """Safely evaluate a sum/difference of attributes and numeric literals.

        Supports the documented case of combining CPU times, e.g.
        ``RemoteUserCpu+RemoteSysCpu``. Anything beyond ``+``/``-`` of
        attributes or numbers is not evaluated and returns ``None``.
        """
        compact = expr.replace(" ", "")
        if not _SUM_EXPR_RE.match(compact):
            self.logger.warning(
                f"Unsupported expression class_ad {expr!r}; only sums/differences "
                f"of attributes and numbers are evaluated. Skipping."
            )
            return None
        total = 0.0
        for term in re.findall(r"[+-]?[A-Za-z0-9_.]+", compact):
            sign = -1.0 if term[0] == "-" else 1.0
            name = term.lstrip("+-")
            if re.fullmatch(r"\d+(?:\.\d+)?", name):
                total += sign * float(name)
            else:
                value = job.get(name)
                if not isinstance(value, (int, float)) or isinstance(value, bool):
                    return None
                total += sign * float(value)
        return int(total) if total.is_integer() else total

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
        for key in ["CompletionDate", "EpochWriteDate", "EnteredCurrentStatus"]:
            if key in job and job[key] not in ("undefined", 0):
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
