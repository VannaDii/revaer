"""BusyBox system accounts used only while preparing the Alpine runtime image."""

import grp
import pwd
import re
from dataclasses import dataclass

from ..errors import ToolingError
from .base import ExternalTool


@dataclass(frozen=True)
class Account:
    uid: int
    gid: int


class AddGroup(ExternalTool):
    version_args = ("--help",)

    def ensure_system(self, name: str) -> int:
        account_name(name)
        try:
            return grp.getgrnam(name).gr_gid
        except KeyError:
            self._invoke(("-S", name))
        try:
            return grp.getgrnam(name).gr_gid
        except KeyError as error:
            raise ToolingError(
                "System group creation did not publish its account record"
            ) from error


class AddUser(ExternalTool):
    version_args = ("--help",)

    def ensure_system(self, name: str, group: str, gid: int) -> Account:
        account_name(name)
        account_name(group)
        try:
            user = pwd.getpwnam(name)
        except KeyError:
            self._invoke(("-S", name, "-G", group))
            try:
                user = pwd.getpwnam(name)
            except KeyError as error:
                raise ToolingError(
                    "System user creation did not publish its account record"
                ) from error
        if user.pw_gid != gid or user.pw_uid == 0 or gid == 0:
            raise ToolingError("Runtime system account has unexpected root/group ownership")
        return Account(user.pw_uid, user.pw_gid)


def account_name(value: str) -> None:
    if not re.fullmatch(r"[a-z][a-z0-9_-]*", value):
        raise ToolingError("System account must use a lowercase name")
