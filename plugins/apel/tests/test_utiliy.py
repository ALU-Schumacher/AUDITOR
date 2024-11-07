import pathlib
import secrets

import pytest

from auditor_apel_plugin.utiliy import write_transaction


CONTENT = [
    "Hello World!",
    "  \t  \n \t\n",
    "",
    secrets.token_hex(),
    secrets.token_hex(4096),
    "\n".join([secrets.token_hex(1025) for _ in range(9)]),
]
# Some content is very long and randomized, parametrize over indizes to keep tests manageable
CONTENT_IDXS = range(len(CONTENT))


@pytest.mark.parametrize("initial_idx", CONTENT_IDXS)
@pytest.mark.parametrize("final_idx", CONTENT_IDXS)
def test_write_transaction(tmp_path: pathlib.Path, initial_idx: int, final_idx: int):
    initial, final = CONTENT[initial_idx], CONTENT[final_idx]
    file_path = tmp_path / "test_file.txt"
    file_path.write_text(initial)
    with write_transaction(file_path) as out_stream:
        for line in final.splitlines(keepends=True):
            out_stream.write(line)
            assert file_path.read_text() == initial
    assert file_path.read_text() == final


@pytest.mark.parametrize("initial_idx", CONTENT_IDXS)
@pytest.mark.parametrize("final_idx", CONTENT_IDXS)
@pytest.mark.parametrize(
    "failure",
    [KeyError, EOFError, IOError, GeneratorExit, SystemExit],
)
def test_write_transaction_failure(
    tmp_path: pathlib.Path,
    initial_idx: int,
    final_idx: int,
    failure: "type[BaseException]",
):
    initial, final = CONTENT[initial_idx], CONTENT[final_idx]
    file_path = tmp_path / "test_file.txt"
    file_path.write_text(initial)
    with write_transaction(file_path) as out_stream, pytest.raises(failure):
        for line in final.splitlines(keepends=True):
            out_stream.write(line)
            assert file_path.read_text() == initial
        raise failure
    assert file_path.read_text() == final
