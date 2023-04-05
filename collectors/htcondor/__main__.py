import asyncio

from time import sleep

from collector import CondorHistoryCollector
from cli import CLI
from config import Config


if __name__ == "__main__":

    args = CLI.parse_args()
    config = Config(args)

    collector = CondorHistoryCollector(config)

    while True:
        asyncio.run(collector.run())
        if config.one_shot:
            break
        sleep(int(config.interval))
