class Error(Exception):
    pass


class RecordExistsError(Error):
    def __init__(self, record_id: str, site_id: str):
        self.record_id = record_id
        self.site_id = site_id


class RecordDoesNotExistError(Error):
    def __init__(self, record_id: str, site_id: str):
        self.record_id = record_id
        self.site_id = site_id


class InsufficientParametersError(Error):
    pass
