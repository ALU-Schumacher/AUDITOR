from collections.abc import Sequence
from typing import Any, Optional, Union

Keys = Optional[Sequence[Union[str, int]]]
Config = dict[Union[str, int], Any]
