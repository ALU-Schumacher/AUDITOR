import asyncio
from time import sleep

from .cli import CLI
from .collector import CondorHistoryCollector
from .config import Config


def main():
    args = CLI.parse_args()
    config = Config(args)

    collector = CondorHistoryCollector(config)

    while True:
        asyncio.run(collector.run())
        if config.one_shot:
            break
        sleep(config.interval)
