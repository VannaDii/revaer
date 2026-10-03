"""A child-process fixture for scanner output failures; it performs no analysis."""

import os
import sys
from pathlib import Path


def main() -> int:
    case = os.environ["RV_TEST_SCANNER_CASE"]
    if "--version" in sys.argv:
        print("unrecognized scanner" if case == "bad-version" else "SonarScanner CLI 8.1.0.6389")
        return 0
    if len(sys.argv) != 1:
        raise ValueError("scanner fixture must receive no command-line overrides")
    if case == "empty-log":
        return 0
    print("Scanner fixture received " + os.environ["SONAR_TOKEN"])
    if case == "exit-failure":
        return 7
    if case == "warning":
        print("\x1b[33mWARN\x1b[0m fixture analysis warning")
    root = Path(".scannerwork")
    root.mkdir(exist_ok=True)
    if case != "missing-task":
        text = "ceTaskId=owned-task\n"
        if case == "duplicate-task":
            text += "ceTaskId=another-task\n"
        (root / "report-task.txt").write_text(text)
    if case != "missing-report":
        report = root / "scanner-report"
        report.mkdir(exist_ok=True)
        (report / "metadata.pb").write_bytes(b"\x00complete submitted report\xff")
    return 0


if __name__ == "__main__":
    sys.exit(main())
