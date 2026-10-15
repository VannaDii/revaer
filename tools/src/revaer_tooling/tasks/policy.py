"""Repository policy checks operate on tracked source and fail closed."""

import fnmatch
import re

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..policy.advisories import advisory_findings
from ..policy.workflows import workflow_findings
from .base import Task
from .compliance import MediaComplianceGuardrails
from .workflows import SonarPolicy, WorkflowPolicy


def matching_lines(source: str, pattern: str, insensitive: bool = False) -> tuple[int, ...]:
    expression = re.compile(pattern, re.IGNORECASE if insensitive else 0)
    return tuple(
        index for index, line in enumerate(source.splitlines(), 1) if expression.search(line)
    )


def rust_findings(path: str, source: str) -> list[str]:
    generated_doc_exception = (
        '#[allow(clippy::missing_errors_doc)]\n#[cxx::bridge(namespace = "revaer")]\n'
    )
    authorized_lines: tuple[int, ...] = ()
    if (
        path == "crates/revaer-torrent-libt/src/ffi/bridge.rs"
        and source.count(generated_doc_exception) == 1
    ):
        authorized_lines = (source[: source.index(generated_doc_exception)].count("\n") + 1,)
    rules: list[tuple[str, str, bool]] = [
        (r"#!?\[(allow|expect)\s*\(", "source-level lint suppression", False),
        (r"todo!|unimplemented!", "authored stub", False),
    ]
    if not path.startswith("crates/revaer-data/src/") and path not in (
        "crates/revaer-test-support/src/postgres.rs",
        "crates/revaer-test-support/tests/integration.rs",
    ):
        rules.append(
            (
                r"sqlx::query(_as|_scalar)?|query!|query_as!|query_scalar!",
                "query outside data boundary",
                False,
            )
        )
    if not (path.startswith("crates/") and "/src/" in path and path.endswith("/tests.rs")):
        rules.append(
            (
                r"(?:^|[^a-z_])(INSERT\s+INTO|UPDATE\s+(?:\"[^\"]+\"|[a-z_][\w\".]*)\s+SET\s+[^=\s][^=]*=|DELETE\s+FROM|CREATE\s+TABLE|ALTER\s+TABLE|DROP\s+TABLE|TRUNCATE\s+TABLE)(?:[^a-z_]|$)",
                "inline DDL/DML",
                True,
            )
        )
    if path != "crates/revaer-torrent-libt/src/ffi.rs" and not path.startswith(
        "crates/revaer-torrent-libt/src/ffi/"
    ):
        rules.extend(
            (
                (r"\bcatch_unwind\b", "catch_unwind outside FFI", False),
                (r"\bunsafe(?:\s+extern|\s+impl|\s+fn|\s*\{)", "unsafe outside FFI", False),
            )
        )
    return [
        f"{path}:{line}: {title}"
        for pattern, title, insensitive in rules
        for line in matching_lines(source, pattern, insensitive)
        if title != "source-level lint suppression" or line not in authorized_lines
    ]


class SourcePolicy(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        failures: list[str] = []
        for relative in context.tools.git.files(include_untracked=True):
            path = context.root / relative
            if not path.is_file():
                continue
            if relative.endswith(".rs") and relative.startswith(("crates/", "tests/", "scripts/")):
                failures.extend(rust_findings(relative, context.fs.read(path)))
            if relative.startswith((".github/workflows/", ".github/actions/")) and path.suffix in (
                ".yaml",
                ".yml",
            ):
                failures.extend(
                    workflow_findings(relative, context.fs.read(path), context.command_names)
                )
            if relative.startswith(
                (
                    "crates/",
                    "tests/",
                    "scripts/",
                    "release/",
                    "tools/",
                    ".github/actions/",
                    ".github/workflows/",
                )
            ) and path.suffix in (
                ".rs",
                ".py",
                ".sh",
                ".rb",
                ".js",
                ".ts",
                ".yaml",
                ".yml",
                ".cpp",
                ".h",
                ".sql",
            ):
                failures.extend(
                    f"{relative}:{line}: Sonar suppression comments are forbidden"
                    for line in matching_lines(context.fs.read(path), r"NO[S]ONAR")
                )
        if failures:
            raise ToolingError("Policy guardrail failed:\n" + "\n".join(failures))
        return TaskResult("Policy checks passed")


class AdvisoryPolicy(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        failures = advisory_findings(
            context.fs.read(context.root / ".secignore"),
            context.fs.read(context.root / "deny.toml"),
        )
        if failures:
            raise ToolingError("Advisory ignores are forbidden:\n" + "\n".join(failures))
        return TaskResult("Advisory exception policy passed")


class Policy(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        SourcePolicy.run(context)
        AdvisoryPolicy.run(context)
        MediaComplianceGuardrails.run(context)
        SonarPolicy.run(context)
        WorkflowPolicy.run(context)
        return TaskResult("Policy checks passed")


def drift_findings(changed: tuple[str, ...]) -> list[str]:
    rules = (
        (
            (
                "tools/**",
                "pyproject.toml",
                "uv.lock",
                "setup.sh",
                "justfile",
                "just/**",
                "config/database-rebaseline.env",
                "scripts/database-rebaseline.rb",
                "scripts/database_rebaseline/**",
                "scripts/workflow_guardrails/**",
                "scripts/*guardrails*",
            ),
            (
                "AGENTS.md",
                ".github/instructions/python.instructions.md",
                ".github/instructions/devops.instructions.md",
            ),
        ),
        (
            (
                ".github/workflows/**",
                ".github/actions/**",
                ".github/build-inputs.env",
                "release/**",
                "Dockerfile",
                "charts/revaer/**",
            ),
            ("AGENTS.md", ".github/instructions/devops.instructions.md"),
        ),
        (
            (
                "tests/**",
                "tools/src/revaer_tooling/e2e/**",
                "tools/src/revaer_tooling/tasks/e2e.py",
            ),
            (
                "AGENTS.md",
                ".github/instructions/devops.instructions.md",
                ".github/instructions/revaer-ui.instructions.md",
            ),
        ),
        (
            ("sonar-project.properties", ".github/workflows/sonar.yml"),
            (
                "AGENTS.md",
                ".github/instructions/devops.instructions.md",
                ".github/instructions/sonarqube_mcp.instructions.md",
            ),
        ),
    )
    failures: list[str] = []
    for patterns, instructions in rules:
        matching = [
            file
            for file in changed
            if any(fnmatch.fnmatchcase(file, pattern) for pattern in patterns)
        ]
        if matching and not set(changed).intersection(instructions):
            failures.append(f"Update {' or '.join(instructions)} for: {', '.join(matching)}")
    return failures


class InstructionDrift(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        base = context.options.base or context.settings.diff_base
        head = context.options.head or context.settings.diff_head
        if base == "0" * 40:
            base = None
        changed = context.tools.git.changed_files(base, head)
        failures = drift_findings(changed)
        if failures:
            raise ToolingError("Instruction drift:\n" + "\n".join(failures))
        return TaskResult("Instruction drift checks passed")


class LanguageLint(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.python.lint()
        context.tools.python.types()
        context.tools.cargo.clippy()
        context.tools.cargo.clippy(production=True)
        return TaskResult()


class Lint(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        Policy.run(context)
        return LanguageLint.run(context)
