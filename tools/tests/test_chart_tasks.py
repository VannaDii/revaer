"""Lint composes mandatory chart checks and stops before packaging on failure."""

import pytest
from revaer_tooling.cli import COMMANDS
from revaer_tooling.context import Context, TaskResult
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.charts import (
    ComplianceChartTest,
    HelmAnnotationTest,
    HelmLint,
    HelmPackage,
    HelmPackageTest,
)
from test_charts import chart_context as chart_context


@pytest.mark.parametrize("media", (False, True))
@pytest.mark.parametrize("failure", (None, "annotations", "packages"))
def test_lint_prerequisites_stop_before_package_replacement(
    chart_context: Context,
    monkeypatch: pytest.MonkeyPatch,
    media: bool,
    failure: str | None,
) -> None:
    if media:
        (chart_context.root / "charts/revaer/values.yaml").write_text("compliance: {}\n")
    calls: list[str] = []

    def record(name: str, context: Context) -> TaskResult:
        calls.append(name)
        if name == "package":
            assert not context.settings.chart.sign
        if name == failure:
            raise ToolingError("fixture prerequisite failed")
        return TaskResult(name)

    monkeypatch.setattr(HelmAnnotationTest, "run", staticmethod(lambda c: record("annotations", c)))
    monkeypatch.setattr(ComplianceChartTest, "run", staticmethod(lambda c: record("compliance", c)))
    monkeypatch.setattr(HelmPackageTest, "run", staticmethod(lambda c: record("packages", c)))
    monkeypatch.setattr(HelmPackage, "run", staticmethod(lambda c: record("package", c)))
    expected = ["annotations", *(["compliance"] if media else []), "packages", "package"]
    if failure is None:
        assert HelmLint.run(chart_context).message == "package"
    else:
        with pytest.raises(ToolingError, match="prerequisite failed"):
            HelmLint.run(chart_context)
        expected = expected[: expected.index(failure) + 1]
    assert calls == expected


def test_chart_prerequisites_are_registered_static_tasks() -> None:
    assert COMMANDS["helm-annotation-test"] == HelmAnnotationTest.run
    assert COMMANDS["helm-package-test"] == HelmPackageTest.run
