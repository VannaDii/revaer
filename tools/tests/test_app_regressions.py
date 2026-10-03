"""Use actual Cargo to verify app-regression selection and failure propagation."""

from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.tasks.testing import UiE2eAppTest
from test_build_tasks import workspace as workspace


@pytest.fixture
def regression_workspace(workspace: Context, monkeypatch: pytest.MonkeyPatch) -> Context:
    crate = workspace.root / "crates/revaer-app"
    with (crate / "Cargo.toml").open("a") as output:
        output.write('[features]\ndefault = ["normal"]\nnormal = []\nunselected = []\n')
    for variable in ("DATABASE_URL", "REVAER_TEST_DATABASE_URL"):
        monkeypatch.setenv(variable, "postgresql://127.0.0.1/disposable")
    (crate / "src/lib.rs").write_text(
        "#[cfg(test)] mod bootstrap { "
        "mod runtime_tests { #[test] fn e2e_launch() { " + body("launch") + " } } "
        "mod compliance_tests { #[test] fn checks() { " + body("compliance") + " } } } "
        '#[test] fn unrelated_must_not_run() { assert!(false, "wrong selection"); }\n'
    )
    (crate / "tests").mkdir()
    (crate / "tests/bootstrap.rs").write_text(
        "#[test] fn bootstrap() { " + body("bootstrap") + " }\n"
    )
    (crate / "tests/unrelated.rs").write_text("#[test] fn unrelated() { assert!(false); }\n")
    return make_context(Options())


def body(group: str) -> str:
    return (
        'assert!(cfg!(feature = "normal")); assert!(!cfg!(feature = "unselected")); '
        'assert_eq!(std::env::var("DATABASE_URL").as_deref(), '
        'Ok("postgresql://127.0.0.1/disposable")); '
        'assert_eq!(std::env::var("REVAER_TEST_DATABASE_URL").as_deref(), '
        'Ok("postgresql://127.0.0.1/disposable")); '
        f'assert!(true, "{group}"); std::fs::write("{group}.ran", "passed").unwrap(); '
    )


def test_native_groups_preserve_features_selection_and_environment(
    regression_workspace: Context,
) -> None:
    assert COMMANDS["ui-e2e-app-test"] == UiE2eAppTest.run
    UiE2eAppTest.run(regression_workspace)
    crate = regression_workspace.root / "crates/revaer-app"
    assert {path.name for path in crate.glob("*.ran")} == {
        "launch.ran",
        "compliance.ran",
        "bootstrap.ran",
    }


@pytest.mark.parametrize("group", ("launch", "compliance", "bootstrap"))
@pytest.mark.parametrize("mode", ("failure", "empty"))
def test_missing_or_failing_groups_stop_the_task(
    regression_workspace: Context,
    group: str,
    mode: str,
) -> None:
    crate = regression_workspace.root / "crates/revaer-app"
    source: Path = crate / ("tests/bootstrap.rs" if group == "bootstrap" else "src/lib.rs")
    text = source.read_text()
    if mode == "failure":
        text = text.replace(f'assert!(true, "{group}")', f'assert!(false, "{group}")')
    elif group == "bootstrap":
        text = "// Intentionally empty integration binary.\n"
    else:
        target = "e2e_launch" if group == "launch" else "compliance_tests"
        text = text.replace(target, "renamed_group")
    source.write_text(text)
    with pytest.raises(CommandError if mode == "failure" else ToolingError):
        UiE2eAppTest.run(regression_workspace)
    order = ("launch", "compliance", "bootstrap")
    assert {path.stem for path in crate.glob("*.ran")} == set(order[: order.index(group)])
