"""Failures crossing the CLI boundary retain a useful exit status."""


class ToolingError(Exception):
    """An actionable failure reported once by the command dispatcher."""

    def __init__(self, message: str, exit_code: int = 1) -> None:
        super().__init__(message)
        self.exit_code = exit_code


class CommandError(ToolingError):
    """An external command failed; any streamed diagnostics precede this summary."""
