class RecordGenerationException(Exception):
    def __init__(self, job, reason=""):
        self.job = job
        self.reason = reason
        super(RecordGenerationException, self).__init__(
            f"Failed to generate record for job {self.job!r}. {self.reason}"
        )


class ConfigError(Exception):
    """Base class for errors in the configuration."""


class MissingConfigEntryError(ConfigError):
    """Exception for missing config entries."""

    def __init__(self, entry):
        self.entry = entry
        super(MissingConfigEntryError, self).__init__(
            f"Missing config entry: {self.entry!r}."
        )


class MalformedConfigEntryError(ConfigError):
    """Exception for malformed config entries."""

    def __init__(self, entry, reason=""):
        self.entry = entry
        self.reason = reason
        super(MalformedConfigEntryError, self).__init__(
            f"Malformed config entry {self.entry!r}{f': {self.reason}' if self.reason else ''}."
        )
