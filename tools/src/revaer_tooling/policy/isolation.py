"""Device provisioning restrictions shared by existing source policy checks."""

import re


def isolation_findings(path: str, source: str) -> list[str]:
    """Reject device provisioning in runtime adapters and workflow invocations."""
    scoped = path.startswith(
        (
            "crates/",
            "tools/src/revaer_tooling/tasks/",
            "tools/src/revaer_tooling/external/",
            ".github/workflows/",
            ".github/actions/",
        )
    )
    if not scoped or "/tests/" in path or path.endswith("/tests.rs"):
        return []
    pattern = (
        r"\b(?:losetup|mkfs(?:\.\w+)?|mknod)\b|/dev/(?:loop\d*|fuse)\b"
        r"|[\"']loop,|\bmount\b.*-o\s+loop\b|--device(?:=|\s)|--privileged\b"
        r"|\bprivileged\s*:\s*true\b|\b(?:SYS_ADMIN|CAP_SYS_ADMIN)\b"
    )
    return [
        f"{path}:{line}: filesystem device provisioning is forbidden"
        for line, text in enumerate(source.splitlines(), 1)
        if re.search(pattern, text)
    ]
