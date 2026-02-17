#!/usr/bin/env python

import argparse
import asyncio
import datetime
import json
from datetime import timedelta, timezone
from logging import Logger
from pathlib import Path
from typing import Any, Dict, List, Optional

import pandas as pd
from dateutil.relativedelta import relativedelta
from pyauditor import AuditorClientBuilder, Operator, QueryBuilder, Value

from auditor_utilization_plugin.config import Config
from auditor_utilization_plugin.email_sender import send_email


def build_query(start: datetime.datetime, end: datetime.datetime) -> str:
    _start = start.astimezone(datetime.timezone.utc)
    _end = end.astimezone(datetime.timezone.utc)
    query_string_start = (
        QueryBuilder()
        .with_stop_time(Operator().gte(Value.set_datetime(_start)))
        .build()
    )
    query_string_end = (
        QueryBuilder().with_stop_time(Operator().lt(Value.set_datetime(_end))).build()
    )
    return query_string_start + "&" + query_string_end


def records_to_df(records: List[Any]) -> pd.DataFrame:
    mylist = []
    for r in records:
        rec = json.loads(r.to_json())
        mylist.append(record_to_dict(rec))
    return pd.DataFrame(mylist)


def record_to_dict(rec):
    my_dict = {}
    for k, v in rec.items():
        if isinstance(v, str) or isinstance(v, int):
            my_dict[k] = v
        elif isinstance(v, list):
            for e in v:
                my_dict[e["name"]] = e["amount"]
                if e["scores"]:
                    my_dict[e["scores"][0]["name"]] = e["scores"][0]["value"]
        elif isinstance(v, dict):
            for n, a in v.items():
                my_dict[n] = a[0]
        else:
            pass
    return my_dict


def rename_user(vo: str) -> str:
    vo_to_name = vo.split("/")
    if len(vo_to_name) >= 4:
        name = vo_to_name[1]
        if "NULL" not in vo_to_name[2]:
            name = name + "-" + vo_to_name[2].split("=")[-1]
    return name


def map_user_name(
    df: pd.DataFrame, col_name: str, group_list: List[str]
) -> pd.DataFrame:
    names = []
    for voms in df[col_name]:
        for name in group_list:
            if name in voms:
                voms = name
                break
        names.append(voms)
    df["names"] = names
    return df


def get_stats_by_user(
    df_in: pd.DataFrame, co2: float, grouped: str, grouped_list: List[str]
) -> Dict[str, List[Any]]:
    data: Dict[str, List[Any]] = {
        "user": [],
        "khs23h": [],
        "cpu_eff": [],
        "corehours": [],
        "power [kWh]": [],
        "co2 [kg]": [],
    }
    for user in df_in[grouped].dropna().unique():
        df = df_in[df_in[grouped].str.contains(user)]
        if "/" in user:
            user = rename_user(user)
        wall_work = (df.HEPscore23 * df.Cores * df.runtime / 3600.0).sum() / 1000.0
        wall_time = (df.Cores * df.runtime / 3600.0).sum() / 1000.0
        cpu_eff = df.TotalCPU.sum() / (df.runtime * df.Cores).sum()
        power = (df.watt_per_core * df.Cores * df.runtime / 3600.0).sum() / 1000.0
        data["user"].append(user)
        data["khs23h"].append(wall_work)
        data["cpu_eff"].append(cpu_eff)
        data["corehours"].append(wall_time)
        data["power [kWh]"].append(power)
        data["co2 [kg]"].append(power * co2)
    return data


def categorize_power(site_name: str, power_dict: Dict[str, float]) -> Optional[float]:
    return power_dict.get(site_name, None)


async def generate_utilization_report(
    logger: Logger,
    config: Config,
    args: argparse.Namespace,
    client: AuditorClientBuilder,
    host: str,
):
    while True:
        today = datetime.datetime.now(datetime.timezone.utc)

        if args.month and args.year:
            start_day = datetime.datetime(
                args.year, args.month, 1, 0, 0, tzinfo=timezone.utc
            )
            end_day = start_day + relativedelta(months=+1)
            if today <= end_day:
                end_day = today

        else:
            prev_month = today - relativedelta(months=1)

            # Set to the first day of that month (keeping UTC)
            start_day = datetime.datetime(
                prev_month.year, prev_month.month, 1, tzinfo=timezone.utc
            )
            end_day = datetime.datetime(
                today.year, today.month, 1, tzinfo=datetime.timezone.utc
            )

        month = args.month or start_day.month
        year = args.year or start_day.year

        loop_day = start_day

        total_records = 0

        daily_summaries = []

        while end_day > loop_day:
            next_day = loop_day + timedelta(days=1)
            query = build_query(loop_day, next_day)

            try:
                records = client.advanced_query(query)
            except Exception as e:
                logger.exception(
                    f"Error during querying records for {host} on {loop_day}: {e}"
                )
                raise

            if len(records) == 0:
                logger.warning(f"No records on this day {loop_day}")
                loop_day = next_day
                continue

            total_records += len(records)

            logger.info(
                f"Total Number of Records {len(records)} for {loop_day.date()} to {next_day.date()}"
            )
            df = records_to_df(records)

            if config.cluster.watt_per_core:
                power_dict = config.cluster.watt_per_core["site"]
                logger.info(f"power_dict {power_dict}")
                df["watt_per_core"] = df["site"].apply(
                    categorize_power, power_dict=power_dict
                )
            else:
                df["watt_per_core"] = config.utilization.watt_per_core

            if config.utilization.grouped_list:
                raw_col = config.utilization.groupedby
                df = map_user_name(df, raw_col, config.utilization.grouped_list)
                mapped_col = "names"
            else:
                mapped_col = config.utilization.groupedby

            summary_data = get_stats_by_user(
                df,
                config.utilization.co2_per_kwh,
                mapped_col,
                config.utilization.grouped_list,
            )
            df_sum = pd.DataFrame(summary_data)
            df_sum["date"] = loop_day.date()
            daily_summaries.append(df_sum)

            loop_day = next_day

        if daily_summaries:
            df_sum_agg = compute_aggregation(daily_summaries, logger)

            if config.email.enable_email_report:
                await mailing_list(
                    config.email.smtp_server, config.email.smtp_port, df_sum_agg, logger
                )

            write_to_csv(
                df_sum_agg,
                host,
                month,
                year,
                logger,
                config.utilization.file_name,
                config.utilization.file_path,
            )

        logger.info(f"total records -->> {total_records}")

        if config.oneshot or args.oneshot:
            logger.info("one-shot finished")
            quit()

        await asyncio.sleep(config.utilization.interval)


def compute_aggregation(
    daily_summaries: List[pd.DataFrame], logger: Logger
) -> pd.DataFrame:
    df_sum_total = pd.concat(daily_summaries, ignore_index=True)

    # Aggregate over time (sum up all days)
    df_sum_agg = df_sum_total.groupby("user", as_index=False).agg(
        {
            "khs23h": "sum",
            "cpu_eff": "mean",
            "corehours": "sum",
            "power [kWh]": "sum",
            "co2 [kg]": "sum",
        }
    )

    logger.info("\n=== Aggregated Summary ===")
    logger.info(df_sum_agg)

    return df_sum_agg


async def mailing_list(
    smtp_server: str, smtp_port: int, df_sum_agg: pd.DataFrame, logger: Logger
):
    return await send_email(smtp_server, smtp_port, df_sum_agg, logger)


def write_to_csv(
    df_sum_agg: pd.DataFrame,
    host: str,
    month: int,
    year: int,
    logger: Logger,
    file_name: Optional[str],
    file_path: Optional[str],
):
    filename = f"{file_name}_{host}_{month}_{year}.csv"
    try:
        base_dir = Path.cwd() if not file_path else Path(file_path)

        base_dir.mkdir(parents=True, exist_ok=True)

        full_path = base_dir / filename

        df_sum_agg.to_csv(full_path, index=False)

        logger.info(f"Successfully wrote file: {full_path}")

    except Exception as e:
        logger.exception(f"Error writing CSV file {filename}: {e}")
        raise
