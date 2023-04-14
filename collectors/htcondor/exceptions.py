class RecordGenerationException(Exception):
    def __init__(self, job, reason=""):
        self.job = job
        self.reason = reason
        super(RecordGenerationException, self).__init__(
            f"Failed to generate record for job {self.job!r}. {self.reason}"
        )
