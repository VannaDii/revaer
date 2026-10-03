"""Mandatory empty advisory lists, shared by policy and audit task boundaries."""

import tomllib


def advisory_findings(secignore: str, deny: str) -> list[str]:
    failures: list[str] = []
    if secignore:
        failures.append(".secignore must remain empty")
    try:
        configuration = tomllib.loads(deny)
    except tomllib.TOMLDecodeError as error:
        return [*failures, f"deny.toml is invalid: {error}"]
    advisories = configuration.get("advisories")
    if not isinstance(advisories, dict) or advisories.get("ignore") != []:
        failures.append("deny.toml [advisories].ignore must be exactly []")
    return failures
