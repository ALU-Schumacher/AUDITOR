#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

from typing import List

from pydantic import BaseModel


class Message(BaseModel):
    message_header: str = ""
    create_sql: List[str] = []
    group_by: List[str] = []
    store_as: List[str] = []
    message_fields: List[str] = []
    aggr_fields: List[str] = []


class SummaryMessage(Message):
    message_header: str = "APEL-summary-job-message: v0.3\n"

    create_sql: List[str] = [
        "Site TEXT NOT NULL",
        "Month INT NOT NULL",
        "Year INT NOT NULL",
        "StopTime INT NOT NULL",
        "WallDuration INT NOT NULL",
        "RecordID TEXT UNIQUE NOT NULL",
    ]

    group_by: List[str] = ["Site", "Month", "Year", "VO", "SubmitHost", "Processors"]

    store_as: List[str] = [
        "COUNT(RecordID) as NumberOfJobs",
        "SUM(WallDuration) as WallDuration",
        "SUM(NormalisedWallDuration) as NormalisedWallDuration",
        "SUM(CpuDuration) as CpuDuration",
        "SUM(NormalisedCpuDuration) as NormalisedCpuDuration",
        "MIN(StopTime) as EarliestEndTime",
        "MAX(StopTime) as LatestEndTime",
    ]

    message_fields: List[str] = [
        "Site",
        "Month",
        "Year",
        "GlobalUserName",
        "VO",
        "VOGroup",
        "VORole",
        "SubmitHost",
        "Infrastructure",
        "NodeCount",
        "Processors",
        "EarliestEndTime",
        "LatestEndTime",
        "WallDuration",
        "CpuDuration",
        "NormalisedWallDuration",
        "NormalisedCpuDuration",
        "NumberOfJobs",
    ]

    aggr_fields: List[str] = [
        "WallDuration",
        "CpuDuration",
        "NormalisedWallDuration",
        "NormalisedCpuDuration",
        "NumberOfJobs",
    ]


class SyncMessage(Message):
    message_header: str = "APEL-sync-message: v0.1\n"

    create_sql: List[str] = [
        "Site TEXT NOT NULL",
        "Month INT NOT NULL",
        "Year INT NOT NULL",
        "SubmitHost TEXT NOT NULL",
        "RecordID TEXT UNIQUE NOT NULL",
    ]

    group_by: List[str] = ["Site", "Month", "Year", "SubmitHost"]

    store_as: List[str] = ["COUNT(RecordID) as NumberOfJobs"]

    message_fields: List[str] = ["Site", "SubmitHost", "NumberOfJobs", "Month", "Year"]

    aggr_fields: List[str] = ["NumberOfJobs"]


class PluginMessage(Message):
    group_by: List[str] = ["Site", "Month", "Year"]

    aggr_fields: List[str] = [
        "NumberOfJobs",
        "WallDuration",
        "CpuDuration",
        "NormalisedWallDuration",
        "NormalisedCpuDuration",
    ]
