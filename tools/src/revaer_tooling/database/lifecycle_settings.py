"""Explicit credentials for disposable single-init application databases."""

from collections.abc import Mapping
from dataclasses import dataclass, field


@dataclass(frozen=True)
class LifecycleSettings:
    container: str
    admin: str
    password: str = field(repr=False)
    runtime_password: str = field(repr=False)


def load_lifecycle_settings(environment: Mapping[str, str]) -> LifecycleSettings:
    # Unrelated commands need no test-service credentials. The consuming task
    # validates the complete selection before making any Docker call.
    return LifecycleSettings(
        environment.get("PG_CONTAINER", ""),
        environment.get("REVAER_LOCAL_DB_USER", ""),
        environment.get("REVAER_LOCAL_DB_PASSWORD", ""),
        environment.get("REVAER_TEST_RUNTIME_PASSWORD", ""),
    )
