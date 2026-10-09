"""GitHub Actions structure and command ownership, independent of YAML layout.

Installation actions and artifact transport remain declarative workflow steps.
Executable project behavior must resolve to the injected rv command registry.
This prevents a workflow from bypassing local gates or hiding policy in shell or
JavaScript blocks. Repository-specific job/evidence contracts build on this
structural validator.
"""

import re
import shlex
from collections.abc import Mapping

from ..errors import ToolingError
from .formats import Document, Value, yaml_document

EXTERNAL_ACTION = re.compile(r"[^./][^/]*/[^@\s]+@[0-9a-f]{40}")
DOCKER_ACTION = re.compile(r"docker://[^\s@]+@sha256:[0-9a-f]{64}")
FORBIDDEN_EXECUTORS = (
    "SonarSource/sonarqube-scan-action@",
    "aquasecurity/trivy-action@",
    "docker/build-push-action@",
    "actions/github-script@",
)
MAX_JOB_TIMEOUT = 180
SETUP_PATH = ".github/actions/setup-revaer/action.yml"


def workflow_findings(path: str, source: str, commands: frozenset[str]) -> list[str]:
    try:
        document = yaml_document(source, path)
    except ToolingError as error:
        return [str(error)]
    return WorkflowPolicy(path, commands).validate(document)


def mapping(value: Value | None) -> Document:
    """Return a mapping view for partial-path checks; callers enforce presence."""
    return value if isinstance(value, dict) else {}


def steps(job: Document) -> list[Document]:
    value = job.get("steps")
    return [entry for entry in value if isinstance(entry, dict)] if isinstance(value, list) else []


def named_steps(job: Document) -> dict[str, Document]:
    return {
        entry["name"]: entry
        for entry in steps(job)
        if isinstance(entry.get("name"), str) and isinstance(entry["name"], str)
    }


def needs(job: Document) -> tuple[str, ...]:
    value = job.get("needs", [])
    if isinstance(value, str):
        return (value,)
    if isinstance(value, list) and all(isinstance(item, str) for item in value):
        return tuple(item for item in value if isinstance(item, str))
    raise ToolingError("job needs must be a string or sequence of strings")


class WorkflowPolicy:
    """One document's diagnostics, collected without swallowing invalid nodes."""

    def __init__(self, path: str, commands: frozenset[str]) -> None:
        self.path = path
        self.commands = commands
        self.failures: list[str] = []

    def error(self, owner: str, message: str) -> None:
        self.failures.append(f"{self.path}: {owner}: {message}")

    def validate(self, document: Document) -> list[str]:
        if "/actions/" in self.path:
            runs = mapping(document.get("runs"))
            if runs.get("using") != "composite":
                self.error("action", "authored actions must use the composite runner")
            self.validate_steps("composite action", runs.get("steps"))
        else:
            self.validate_workflow(document)
        return self.failures

    def validate_workflow(self, document: Document) -> None:
        self.validate_permissions("workflow", document)
        jobs = document.get("jobs")
        if not isinstance(jobs, dict) or not jobs:
            self.error("workflow", "jobs must be a non-empty mapping")
            return
        for name, value in jobs.items():
            owner = f"job {name}"
            if not isinstance(value, dict):
                self.error(owner, "must be a mapping")
                continue
            self.validate_failure(owner, value)
            self.validate_condition(owner, value)
            self.validate_permissions(owner, value)
            self.validate_postgres(owner, value)
            if "uses" in value:
                self.validate_uses(owner, value["uses"])
                if "steps" in value:
                    self.error(owner, "reusable jobs must not define steps")
            else:
                timeout = value.get("timeout-minutes")
                if (
                    not isinstance(timeout, str)
                    or not re.fullmatch(r"[1-9][0-9]*", timeout)
                    or int(timeout) > MAX_JOB_TIMEOUT
                ):
                    self.error(owner, f"timeout-minutes must be between 1 and {MAX_JOB_TIMEOUT}")
                self.validate_steps(owner, value.get("steps"))
        self.validate_dependencies(jobs)

    def validate_steps(self, owner: str, values: Value | None) -> None:
        if not isinstance(values, list) or not values:
            self.error(owner, "steps must be a non-empty sequence")
            return
        names: set[str] = set()
        identifiers: set[str] = set()
        prepared = False
        for index, step in enumerate(values, 1):
            label = f"{owner} step {index}"
            if not isinstance(step, dict):
                self.error(label, "must be a mapping")
                continue
            for key, seen in (("name", names), ("id", identifiers)):
                value = step.get(key)
                if isinstance(value, str) and value:
                    if value in seen:
                        self.error(label, f"duplicate nonempty step {key} {value!r}")
                    seen.add(value)
            self.validate_failure(label, step)
            self.validate_condition(label, step)
            if ("run" in step) == ("uses" in step):
                self.error(label, "must define exactly one of uses or run")
            if "uses" in step:
                self.validate_uses(label, step["uses"])
                if step["uses"] == "./.github/actions/setup-revaer" and "if" not in step:
                    prepared = True
            if "run" in step:
                if self.path.startswith(".github/workflows/") and not prepared:
                    self.error(label, "run requires an earlier unconditional setup-revaer step")
                self.validate_run(label, step["run"])
            if step.get("uses") == "./.github/actions/setup-revaer":
                # The full coverage environment includes cold builds of every
                # pinned Cargo tool; ordinary setup keeps its shorter bound.
                timeout = (
                    "40" if mapping(step.get("with", {})).get("apt-profile") == "coverage" else "20"
                )
                if step.get("timeout-minutes") != timeout:
                    self.error(label, f"setup-revaer requires timeout-minutes: {timeout}")

    def validate_uses(self, owner: str, value: Value) -> None:
        if not isinstance(value, str) or not (
            (re.fullmatch(r"\./[^\s]+", value) and ".." not in value.split("/"))
            or EXTERNAL_ACTION.fullmatch(value)
            or DOCKER_ACTION.fullmatch(value)
        ):
            self.error(owner, f"unpinned action {value!r}; require a full SHA or Docker digest")
        if isinstance(value, str) and value.startswith(FORBIDDEN_EXECUTORS):
            self.error(owner, "project gates and embedded logic must execute through rv")

    def validate_run(self, owner: str, value: Value) -> None:
        if not isinstance(value, str) or not value.strip():
            self.error(owner, "run must be a non-empty scalar")
            return
        if re.search(r"\$\{\{\s*(inputs\.|github\.event\.)", value):
            self.error(owner, "untrusted expression in run block; pass it through env")
        if "-Dsonar." in value or "--define sonar." in value:
            self.error(owner, "Sonar overrides belong only in sonar-project.properties")
        try:
            command = shlex.split(value, comments=False)
        except ValueError as error:
            self.error(owner, f"invalid command quoting: {error}")
            return
        # A single simple invocation is sufficient. Shell expansion, pipelines,
        # redirects and command substitution add another executable language.
        if re.search(r"[;|&<>`\n]|\$\(", value.strip()):
            self.error(owner, "run must contain one CLI invocation without shell logic")
            return
        if command == ["uv", "sync", "--locked"] and self.path == SETUP_PATH:
            return
        if command[:5] == ["uv", "run", "--locked", "--", "rv"]:
            command = command[4:]
        if command and command[0] in ("rv", ".venv/bin/rv"):
            if len(command) > 1 and (
                command[1] in self.commands or " ".join(command[1:3]) in self.commands
            ):
                return
            self.error(owner, f"unknown rv command: {' '.join(command[1:3])}")
            return
        self.error(owner, "project commands must run through rv")

    def validate_failure(self, owner: str, node: Document) -> None:
        if "continue-on-error" in node and node["continue-on-error"] != "false":
            self.error(owner, "continue-on-error must not conceal failure")

    def validate_condition(self, owner: str, node: Document) -> None:
        if "if" in node:
            value = node["if"]
            if not isinstance(value, str) or not value.strip() or "\n" in value:
                self.error(owner, "if must be one non-empty scalar")

    def validate_permissions(self, owner: str, node: Document) -> None:
        if "permissions" not in node:
            return
        permissions = node["permissions"]
        if not isinstance(permissions, dict) or not permissions:
            self.error(owner, "permissions must be a non-empty mapping")
            return
        for key, value in permissions.items():
            if not re.fullmatch(r"[a-z-]+", key) or value not in ("none", "read", "write"):
                self.error(owner, f"permission {key} must be none, read, or write")

    def validate_postgres(self, owner: str, node: Document) -> None:
        service = mapping(mapping(node.get("services")).get("postgres"))
        if not service:
            return
        environment = mapping(service.get("env"))
        values = tuple(
            environment.get(key) for key in ("POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB")
        )
        if not all(isinstance(value, str) and value for value in values):
            self.error(owner, "Postgres service must define user, password, and database")
            return
        user, password, database = values
        active = mapping(node.get("env"))
        expected = f"postgres://{user}:{password}@localhost:5432/{database}"
        for key in ("REVAER_TEST_DATABASE_URL", "DATABASE_URL"):
            if active.get(key) != expected:
                self.error(owner, f"{key} must exactly match its Postgres service")
        if "E2E_DB_ADMIN_URL" in active and active["E2E_DB_ADMIN_URL"] != (
            f"postgres://{user}:{password}@localhost:5432/postgres"
        ):
            self.error(owner, "E2E_DB_ADMIN_URL must exactly match its Postgres service")

    def validate_dependencies(self, jobs: Mapping[str, Value]) -> None:
        visiting: set[str] = set()
        visited: set[str] = set()

        def visit(name: str) -> None:
            if name in visiting:
                self.error(f"job {name}", "workflow dependency cycle")
                return
            if name in visited:
                return
            visiting.add(name)
            try:
                dependencies = needs(mapping(jobs[name]))
            except ToolingError as error:
                self.error(f"job {name}", str(error))
                dependencies = ()
            if len(set(dependencies)) != len(dependencies):
                self.error(f"job {name}", "duplicate workflow dependency")
            for dependency in dependencies:
                if dependency not in jobs:
                    self.error(f"job {name}", f"missing dependency {dependency}")
                else:
                    visit(dependency)
            visiting.remove(name)
            visited.add(name)

        for name in jobs:
            visit(name)
