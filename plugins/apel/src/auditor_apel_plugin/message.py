#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

from pydantic import BaseModel


class Message(BaseModel):
    message_header: str = ""
    create_sql: list[str] = []
    group_by: list[str] = []
    store_as: list[str] = []
    message_fields: list[str] = []
    aggr_fields: list[str] = []


class SummaryMessage(Message):
    message_header: str = "APEL-normalised-summary-message: v0.4\n"

    create_sql: list[str] = [
        "Site TEXT NOT NULL",
        "Month INT NOT NULL",
        "Year INT NOT NULL",
        "StopTime INT NOT NULL",
        "WallDuration INT NOT NULL",
        "RecordID TEXT UNIQUE NOT NULL",
    ]

    group_by: list[str] = ["Site", "Month", "Year", "VO", "SubmitHost", "Processors"]

    store_as: list[str] = [
        "COUNT(RecordID) as NumberOfJobs",
        "SUM(WallDuration) as WallDuration",
        "SUM(NormalisedWallDuration) as NormalisedWallDuration",
        "SUM(CpuDuration) as CpuDuration",
        "SUM(NormalisedCpuDuration) as NormalisedCpuDuration",
        "MIN(StopTime) as EarliestEndTime",
        "MAX(StopTime) as LatestEndTime",
    ]

    message_fields: list[str] = [
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

    aggr_fields: list[str] = [
        "WallDuration",
        "CpuDuration",
        "NormalisedWallDuration",
        "NormalisedCpuDuration",
        "NumberOfJobs",
    ]


class SyncMessage(Message):
    message_header: str = "APEL-sync-message: v0.1\n"

    create_sql: list[str] = [
        "Site TEXT NOT NULL",
        "Month INT NOT NULL",
        "Year INT NOT NULL",
        "SubmitHost TEXT NOT NULL",
        "RecordID TEXT UNIQUE NOT NULL",
    ]

    group_by: list[str] = ["Site", "Month", "Year", "SubmitHost"]

    store_as: list[str] = ["COUNT(RecordID) as NumberOfJobs"]

    message_fields: list[str] = ["Site", "SubmitHost", "NumberOfJobs", "Month", "Year"]

    aggr_fields: list[str] = ["NumberOfJobs"]


class PluginMessage(Message):
    group_by: list[str] = ["Site", "Month", "Year"]

    aggr_fields: list[str] = [
        "NumberOfJobs",
        "WallDuration",
        "CpuDuration",
        "NormalisedWallDuration",
        "NormalisedCpuDuration",
    ]
