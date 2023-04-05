import yaml
import utils


class Config(object):

    _config = {
        "interval": 900,
        "log_level": "INFO",
        "log_file": None,
        "class_ads": [
            "GlobalJobId",
            "ClusterId",
            "ProcId",
            "LastMatchTime",
            "EnteredCurrentStatus",
        ],
    }

    def __init__(self, args):

        with open(args.config) as f:
            file = yaml.safe_load(f)

        self._config.update(file)
        self._config.update({k: v for k, v in args.__dict__.items() if v is not None})

        self._config["class_ads"] = list(
            set(self._config["class_ads"]).union(set(utils.extract_values("key", file)))
        )

    def __getattr__(self, attr):
        return self._config[attr]

    def get(self, attr, default=None):
        return self._config.get(attr, default)
