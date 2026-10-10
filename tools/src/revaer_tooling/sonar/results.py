"""Validate the authoritative analysis and its complete published result set."""

import re
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation

from ..errors import ToolingError
from ..json_data import JsonObject, array_value, object_value, string_value
from ..policy.formats import properties
from ..policy.sonar import runtime_source


def task_id(source: str) -> str:
    entries = {entry.key: entry.value for entry in properties(source, "report-task.txt")}
    identifier = entries.get("ceTaskId", "")
    if not re.fullmatch(r"[A-Za-z0-9_-]+", identifier):
        raise ToolingError("report-task.txt must contain one valid ceTaskId")
    return identifier


def analysis_id(record: JsonObject, identifier: str, project: str) -> str:
    task = object_value(record.get("task"))
    if task.get("id") != identifier or task.get("componentKey") != project:
        raise ToolingError("Sonar result does not identify the submitted task and project")
    if task.get("status") != "SUCCESS":
        raise ToolingError("The submitted Sonar task has not completed successfully")
    return string_value(task.get("analysisId"))


@dataclass(frozen=True)
class PublishedResult:
    coverage: str
    lines_to_cover: str
    analysis: str


def published_result(
    records: dict[str, JsonObject], identifier: str, project: str
) -> PublishedResult:
    required = {"ce-task", "measures", "quality-gate", "issues", "all-issues", "hotspots"}
    if records.keys() != required:
        raise ToolingError("Sonar result evidence is incomplete")
    analysis = analysis_id(records["ce-task"], identifier, project)
    measures = array_value(object_value(records["measures"].get("component")).get("measures"))
    values: dict[str, str] = {}
    for name in ("coverage", "line_coverage", "lines_to_cover", "uncovered_lines"):
        candidates = [
            object_value(value).get("value")
            for value in measures
            if object_value(value).get("metric") == name
        ]
        if len(candidates) != 1:
            raise ToolingError(f"Sonar must publish exactly one {name} measure")
        text = string_value(candidates[0])
        try:
            number = Decimal(text)
        except InvalidOperation as error:
            raise ToolingError(f"Sonar {name} is not a number") from error
        if not number.is_finite() or number < 0 or (number == 0 and name != "uncovered_lines"):
            raise ToolingError(
                f"Sonar {name} must be {'nonnegative' if name == 'uncovered_lines' else 'positive'}"
            )
        values[name] = text
    gate = object_value(records["quality-gate"].get("projectStatus"))
    if gate.get("status") != "OK" or gate.get("ignoredConditions") is not False:
        raise ToolingError("Sonar quality gate must pass without ignored conditions")
    issues = records["issues"].get("total")
    hotspots = object_value(records["hotspots"].get("paging")).get("total")
    if type(issues) is not int or issues != 0:
        raise ToolingError("Sonar has new unresolved issues or an invalid issue total")
    _verify_runtime_issues(records["all-issues"], project)
    if type(hotspots) is not int or hotspots != 0:
        raise ToolingError("Sonar has unreviewed hotspots or an invalid hotspot total")
    return PublishedResult(values["coverage"], values["lines_to_cover"], analysis)


def _verify_runtime_issues(backlog: JsonObject, project: str) -> None:
    total = backlog.get("total")
    rows = array_value(backlog.get("issues"))
    if type(total) is not int or total != len(rows):
        raise ToolingError("Sonar production issue search is incomplete")
    for row in rows:
        component = string_value(object_value(row).get("component"))
        if component == project:
            continue
        if not component.startswith(project + ":"):
            raise ToolingError("Sonar issue belongs to another project")
        if runtime_source(component.removeprefix(project + ":")):
            raise ToolingError("Sonar has unresolved active production issues")
