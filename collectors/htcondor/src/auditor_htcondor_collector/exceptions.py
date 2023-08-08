from .custom_types import Keys


class RecordGenerationException(Exception):
    def __init__(self, job: str, reason: str = ""):
        self.job = job
        self.reason = reason
        super(RecordGenerationException, self).__init__(
            f"Failed to generate record for job {self.job!r}. {self.reason}"
        )


class ConfigError(Exception):
    """Base class for errors in the configuration."""


class MissingConfigEntryError(ConfigError):
    """Exception for missing config entries."""

    def __init__(self, keys: Keys):
        self.keys = keys
        super(MissingConfigEntryError, self).__init__(
            f"Missing config entry: {self.keys!r}."
        )


class MissingConfigDependencyError(ConfigError):
    def __init__(self, keys: Keys, dependency: Keys):
        self.keys = keys
        self.dependency = dependency
        super(MissingConfigDependencyError, self).__init__(
            f"Missing config entry {self.keys!r}, "
            f"must be specified if {self.dependency!r} is specified."
        )


class MalformedConfigEntryError(ConfigError):
    """Exception for malformed config entries."""

    def __init__(self, keys: Keys, reason: str = ""):
        self.keys = keys
        self.reason = reason
        super(MalformedConfigEntryError, self).__init__(
            f"Malformed config entry {self.keys!r}"
            f"{f': {self.reason}' if self.reason else ''}."
        )
