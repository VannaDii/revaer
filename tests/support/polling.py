"""Bounded polling for asynchronous API state; transport errors stay visible."""

import time
from collections.abc import Callable


def eventually(predicate: Callable[[], bool], description: str, timeout: float = 5) -> None:
    deadline = time.monotonic() + timeout
    while not predicate():
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise AssertionError(f"Timed out waiting for {description}")
        time.sleep(min(0.1, remaining))
