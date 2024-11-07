import os
import pathlib
from contextlib import contextmanager
from typing import Generator, TextIO


@contextmanager
def write_transaction(
    path: str | os.PathLike[str], **kwargs
) -> Generator[TextIO, None, None]:
    """Open `path` for overwriting but discard the new content if an exception occurs"""
    tmp_path = pathlib.Path(path).parent / f".{pathlib.Path(path).name}.tmp"
    with open(tmp_path, "w", **kwargs) as out_stream:
        yield out_stream
    tmp_path.rename(path)
