import logging
import pathlib
from contextlib import contextmanager
from typing import IO, ContextManager, Dict, Literal, overload

logger = logging.getLogger("apel_plugin")


@overload
def write_transaction(
    path: "str | pathlib.PurePath", mode: Literal["w"] = ..., **kwargs
) -> ContextManager[IO[str]]: ...


@overload
def write_transaction(
    path: "str | pathlib.PurePath", mode: Literal["wb"], **kwargs
) -> ContextManager[IO[bytes]]: ...


@contextmanager
def write_transaction(
    path: "str | pathlib.PurePath", mode: Literal["w", "wb"] = "w", **kwargs
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


def vo_mapping(user: str, vo_dict: Dict[str, str]) -> str:
    for k, v in vo_dict.items():
        if user.startswith(k):
            return v

    logger.warning(f"No VO for user {user} found, will use None")

    return "None"
