"""Observe reset timeout restoration through real nested calls and concurrency.

Only disposable proof databases may receive these temporary observer routines.
The owning composition supplies the connection, executor, timing and identity
sources, and remains responsible for waiting for workers before container cleanup.
"""

import re
from collections.abc import Callable
from concurrent.futures import Executor, Future

from ..errors import ToolingError
from ..external.postgres import QueryArgs, identifier
from ..process import Completed
from .postgres import Connection

RESET = "revaer_config.factory_reset_without_media_defaults_v1()"


class ResetTimeoutProof:
    def __init__(
        self,
        connection: Connection,
        owner: str,
        runtime: str,
        check: Callable[[str, bool], None],
        executor: Executor,
        monotonic: Callable[[], float],
        sleep: Callable[[float], None],
        token: Callable[[], str],
    ) -> None:
        self.connection = connection
        self.owner, self.runtime = identifier(owner), identifier(runtime)
        self.check, self.executor = check, executor
        self.monotonic, self.sleep, self.token = monotonic, sleep, token
        self.transcript: list[str] = []

    def observer(self, body: str) -> None:
        self.connection.sql(
            "CREATE OR REPLACE FUNCTION revaer_system.proof_reset_observer() RETURNS trigger "
            "LANGUAGE plpgsql SET search_path = pg_catalog AS $proof$ BEGIN "
            "RAISE NOTICE 'reset-timeout-inside:%', current_setting('lock_timeout'); "
            f"{body} RETURN NULL; END; $proof$;",
            role=self.owner,
        )

    def outcome(self, query: str, role: str | None = None) -> Completed:
        principal = role or self.runtime
        result = self.connection.docker.query(
            self.connection.container,
            QueryArgs(principal, self.connection.database, "\\set VERBOSITY default\n" + query),
        )
        self.transcript.append(
            f"role={'runtime' if principal == self.runtime else 'owner'}\n"
            f"{result.stdout}{result.stderr}\n"
        )
        if re.search(r"\bWARNING\b", result.stderr):
            raise ToolingError("Reset timeout proof emitted a PostgreSQL warning")
        return result

    def verify(self) -> None:
        self.transcript = ["D2 real reset timeout scope observations\n"]
        primary: BaseException | None = None
        try:
            self.observer("")
            self.connection.sql(
                "CREATE TRIGGER proof_reset_timeout BEFORE TRUNCATE ON public.app_profile "
                "FOR EACH STATEMENT EXECUTE FUNCTION revaer_system.proof_reset_observer()",
                role=self.owner,
            )
            self.success()
            self.nested()
            self.observer("PERFORM 1 / 0;")
            self.restored(
                "division_by_zero", "22012", self.outcome(self.caught("division_by_zero", "22012"))
            )
            self.cancellation()
            self.observer("")
            self.contention()
        except BaseException as error:
            primary = error
            raise
        finally:
            try:
                self.connection.sql(
                    "DROP TRIGGER IF EXISTS proof_reset_timeout ON public.app_profile; "
                    "DROP FUNCTION IF EXISTS revaer_system.proof_reset_observer(); "
                    "DROP FUNCTION IF EXISTS revaer_system.proof_nested_reset();",
                    role=self.owner,
                )
            except (ToolingError, OSError) as cleanup:
                earlier = f"; earlier {type(primary).__name__}: {primary}" if primary else ""
                raise ToolingError(
                    f"Reset observer cleanup failed: {cleanup}{earlier}"
                ) from cleanup
            finally:
                self.connection.fs.write(
                    self.connection.output / "final-reset-timeout-transcript.txt",
                    "".join(self.transcript),
                    0o600,
                )

    def success(self) -> None:
        for role in (self.owner, self.runtime):
            result = self.outcome(
                "SET lock_timeout = '19s'; BEGIN; "
                "SET LOCAL statement_timeout = '120s'; SET LOCAL lock_timeout = '17s'; "
                "SET LOCAL idle_in_transaction_session_timeout = '30s'; "
                "SELECT current_setting('lock_timeout'); "
                f"SELECT {RESET}; "
                "SELECT current_setting('statement_timeout') || ',' || "
                "current_setting('lock_timeout') || ',' || "
                "current_setting('idle_in_transaction_session_timeout'); ROLLBACK; "
                "SELECT current_setting('lock_timeout');",
                role,
            )
            name = "runtime" if role == self.runtime else "owner"
            self.check(
                f"D2 {name} success and transaction rollback restore caller settings",
                result.code == 0
                and [line.strip() for line in result.stdout.splitlines() if line.strip()]
                == ["17s", "2min,17s,30s", "19s"]
                and self.notices(result) == ["NOTICE:  reset-timeout-inside:5s"],
            )

    @staticmethod
    def notices(result: Completed) -> list[str]:
        return [
            line.strip() for line in result.stderr.splitlines() if "reset-timeout-inside:" in line
        ]

    def nested(self) -> None:
        self.connection.sql(
            "CREATE FUNCTION revaer_system.proof_nested_reset() RETURNS text "
            "LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog "
            "SET lock_timeout = '9s' "
            f"AS $proof$ BEGIN PERFORM {RESET}; "
            "RETURN current_setting('lock_timeout'); END; $proof$; "
            "REVOKE ALL ON FUNCTION revaer_system.proof_nested_reset() FROM PUBLIC; "
            f"GRANT EXECUTE ON FUNCTION revaer_system.proof_nested_reset() TO {self.runtime};",
            role=self.owner,
        )
        result = self.outcome(
            "BEGIN; SET LOCAL lock_timeout = '17s'; SELECT revaer_system.proof_nested_reset(); "
            "SELECT current_setting('lock_timeout'); ROLLBACK;"
        )
        self.check(
            "D2 nested reset restores intermediate and outer lock timeouts",
            result.code == 0
            and [line.strip() for line in result.stdout.splitlines()] == ["9s", "17s"]
            and self.notices(result) == ["NOTICE:  reset-timeout-inside:5s"],
        )

    @staticmethod
    def caught(condition: str, state: str) -> str:
        return (
            "BEGIN; SET LOCAL statement_timeout = '15s'; SET LOCAL lock_timeout = '17s'; "
            f"DO $proof$ BEGIN PERFORM {RESET}; RAISE EXCEPTION 'reset unexpectedly succeeded'; "
            f"EXCEPTION WHEN {condition} THEN IF SQLSTATE <> '{state}' THEN RAISE; END IF; "
            "RAISE NOTICE 'reset-timeout-caught:%:%', SQLSTATE, current_setting('lock_timeout'); "
            "END; $proof$; SELECT current_setting('lock_timeout'); ROLLBACK;"
        )

    def restored(self, name: str, state: str, result: Completed) -> None:
        self.check(
            f"D2 {name} restores caller timeout",
            result.code == 0
            and result.stdout.strip() == "17s"
            and "reset-timeout-inside:5s" in result.stderr
            and f"reset-timeout-caught:{state}:17s" in result.stderr,
        )

    def application(self) -> str:
        token = self.token()
        if not re.fullmatch(r"[0-9a-f]{32}", token):
            raise ToolingError("Reset concurrency requires a fresh 128-bit hexadecimal token")
        return "rv-reset-" + token

    @staticmethod
    def backend_filter(application: str) -> str:
        return (
            "datname = current_database() "
            f"AND application_name = '{application}' AND wait_event = 'PgSleep'"
        )

    def wait_backend(self, application: str, worker: Future[Completed]) -> str:
        deadline = self.monotonic() + 10
        while True:
            pid = self.connection.sql(
                "SELECT pid FROM pg_stat_activity WHERE " + self.backend_filter(application),
                role="postgres",
            )
            if pid:
                if not re.fullmatch(r"[1-9][0-9]*", pid):
                    raise ToolingError("Reset observer backend identity is invalid")
                return pid
            if worker.done():
                worker.result()  # Preserve a native worker failure, if present.
                raise ToolingError("Reset worker ended before reaching the observer")
            if self.monotonic() >= deadline:
                raise ToolingError("Reset worker did not reach the observer")
            self.sleep(0.05)

    def cancel(self, application: str, pid: str) -> bool:
        # Recheck identity at cancellation time, rather than trusting a PID alone.
        return (
            self.connection.sql(
                "SELECT pg_cancel_backend(pid) FROM pg_stat_activity WHERE "
                + self.backend_filter(application)
                + f" AND pid = {pid}",
                role="postgres",
            )
            == "t"
        )

    def cancellation(self) -> None:
        self.observer("PERFORM pg_sleep(30);")
        application = self.application()
        worker = self.executor.submit(
            self.outcome,
            f"SET application_name = '{application}'; " + self.caught("query_canceled", "57014"),
        )
        try:
            pid = self.wait_backend(application, worker)
            self.check(
                "D2 cancellation delivered to the exact active reset backend",
                self.cancel(application, pid),
            )
            self.restored("real query cancellation", "57014", worker.result())
        finally:
            # The independent 15-second server timeout bounds observer failure.
            # Always propagate worker errors and join before removing observers.
            worker.result()

    def contention(self) -> None:
        application = self.application()
        # The holder catches only our explicit cancellation, then rolls back.
        # The bounded sleep also releases it if observer discovery fails.
        holder = self.executor.submit(
            self.outcome,
            f"SET application_name = '{application}'; BEGIN; "
            "LOCK TABLE public.app_profile IN ACCESS SHARE MODE; "
            "DO $hold$ BEGIN PERFORM pg_sleep(15); EXCEPTION WHEN query_canceled THEN "
            "RAISE NOTICE 'reset-lock-released'; END; $hold$; ROLLBACK;",
            self.owner,
        )
        pid: str | None = None
        try:
            pid = self.wait_backend(application, holder)
            started = self.monotonic()
            result = self.outcome(self.caught("lock_not_available", "55P03"))
            elapsed = self.monotonic() - started
            self.check(
                "D2 real lock contention retains five-second bound and restores caller",
                result.code == 0
                and result.stdout.strip() == "17s"
                and "reset-timeout-caught:55P03:17s" in result.stderr
                and 4.5 <= elapsed < 10,
            )
        finally:
            try:
                if pid is not None and not holder.done() and not self.cancel(application, pid):
                    raise ToolingError("Reset lock holder cancellation missed its owned backend")
            finally:
                outcome = holder.result()
                if outcome.code != 0 or outcome.stderr.strip() not in (
                    "",
                    "NOTICE:  reset-lock-released",
                ):
                    raise ToolingError("Reset lock holder failed cleanup")
