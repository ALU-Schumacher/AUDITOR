from argparse import ArgumentParser

CLI = ArgumentParser()
CLI.add_argument(
    "-c",
    "--config",
    help="Path to config file.",
    metavar="CONFIG_FILE",
    required=True,
)
CLI.add_argument(
    "-j",
    "--job-id",
    metavar="<CLUSTERID>[.<PROCID>]",
    help="ID of the job, condor_history to invoke with.",
)
CLI.add_argument(
    "-n",
    "--schedd-names",
    metavar="SCHEDD",
    help="Name of the schedd, condor_history to invoke with.",
    action="append",
)
CLI.add_argument(
    "-k",
    "--history-file",
    metavar="HISTORY_FILE",
    help="Path to history file, to read condor_history from.",
)
CLI.add_argument(
    "-l",
    "--log-level",
    help="Log level. Defaults to INFO.",
    choices=["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"],
)
CLI.add_argument(
    "-f",
    "--log-file",
    help="Log file. Defaults to stdout.",
)
CLI.add_argument(
    "-i",
    "--interval",
    help="Interval in seconds between queries. Defaults to 900.",
    type=int,
)
CLI.add_argument(
    "-1",
    "--one-shot",
    help="Run once and exit.",
    action="store_true",
)
