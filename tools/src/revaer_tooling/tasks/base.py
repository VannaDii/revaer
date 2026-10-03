"""Static task contract used by both public commands and internal composition."""

from abc import ABC, abstractmethod

from ..context import Context, TaskResult


class Task(ABC):
    @staticmethod
    @abstractmethod
    def run(context: Context) -> TaskResult:
        """Execute this task with the supplied collaborators."""
