import os
import pathlib
from contextlib import contextmanager
from typing import IO, ContextManager, Literal, overload


@overload
def write_transaction(
    path: str | os.PathLike[str], mode: Literal["w"] = ..., **kwargs
) -> ContextManager[IO[str]]: ...


@overload
def write_transaction(
    path: str | os.PathLike[str], mode: Literal["wb"], **kwargs
) -> ContextManager[IO[bytes]]: ...


@contextmanager
def write_transaction(
    path: str | os.PathLike[str], mode: Literal["w", "wb"] = "w", **kwargs
):
    """Open `path` for overwriting but discard the new content if an exception occurs"""
    tmp_path = pathlib.Path(path).parent / f".{pathlib.Path(path).name}.tmp"
    try:
        with open(tmp_path, mode, **kwargs) as out_stream:
            yield out_stream
    except BaseException:
        tmp_path.unlink(True)
        raise
    else:
        tmp_path.rename(path)
