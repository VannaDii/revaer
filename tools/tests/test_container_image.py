"""Build the unchanged Dockerfile with a tiny real Rust application fixture.

This proves uv provisioning, native package pins, BuildKit mounts and runtime
ownership without claiming that the full Revaer application was built or tested.
Every builder/image belongs to this fixture and cleanup uses those exact names.
"""

import json
import secrets
import shutil
import subprocess
from pathlib import Path

from revaer_tooling.filesystem import FileSystem
from revaer_tooling.images.inputs import BuildInputs
from test_images import docker, selected_builders


def test_real_dockerfile_uses_uv_and_keeps_tooling_out_of_the_runtime(tmp_path: Path) -> None:
    source = Path(__file__).parents[2]
    root = tmp_path / "source"
    root.mkdir()
    for name in (
        "Dockerfile",
        ".dockerignore",
        "pyproject.toml",
        "uv.lock",
        ".python-version",
        ".uv-version",
        "rust-toolchain.toml",
    ):
        shutil.copy(source / name, root / name)
    shutil.copytree(
        source / "tools/src", root / "tools/src", ignore=shutil.ignore_patterns("__pycache__")
    )
    shutil.copy(source / "tools/versions.toml", root / "tools/versions.toml")
    (root / ".github").mkdir()
    shutil.copy(source / ".github/build-inputs.env", root / ".github/build-inputs.env")
    shutil.copytree(source / "release/media-compliance", root / "release/media-compliance")
    (root / "Cargo.toml").write_text(
        '[workspace]\nmembers=["app"]\nresolver="2"\n[workspace.package]\nedition="2024"\n'
    )
    (root / "app/src").mkdir(parents=True)
    (root / "app/Cargo.toml").write_text(
        '[package]\nname="revaer-app"\nversion="0.1.0"\nedition.workspace=true\n'
    )
    (root / "app/src/main.rs").write_text('fn main() { println!("container fixture"); }\n')
    for name in ("docs", "config"):
        (root / name).mkdir()
        (root / name / "README.md").write_text("# Container fixture\n")
    (root / ".venv").mkdir()
    (root / ".venv/must-not-ship").write_text("Unrelated local environment\n")
    subprocess.run(
        ["cargo", "generate-lockfile"], cwd=root, check=True, capture_output=True, timeout=120
    )
    inputs = BuildInputs.load(FileSystem(), root)
    builder = "rv-container-proof-" + secrets.token_hex(8)
    tag = builder + ":fixture"
    selected = selected_builders()
    docker("buildx", "create", "--name", builder, "--driver", "docker-container")
    try:
        log_path = tmp_path / "docker-build.log"
        with log_path.open("w") as log:
            result = subprocess.run(
                [
                    "docker",
                    "buildx",
                    "build",
                    "--builder",
                    builder,
                    "--load",
                    "--progress",
                    "plain",
                    "--tag",
                    tag,
                    ".",
                ],
                cwd=root,
                stdout=log,
                stderr=subprocess.STDOUT,
                check=False,
                timeout=1200,
            )
        assert result.returncode == 0, log_path.read_text()
        assert "warning:" not in log_path.read_text().lower()
        image = json.loads(docker("image", "inspect", tag))[0]
        assert image["Config"]["User"] == "revaer"
        assert image["Config"]["Entrypoint"] == ["/usr/local/bin/revaer-app"]
        assert image["Config"]["Healthcheck"]["Test"] == [
            "CMD",
            "curl",
            "-fsS",
            "http://127.0.0.1:7070/health/full",
        ]
        assert docker("run", "--rm", tag) == "container fixture"
        packages = set(
            docker(
                "run",
                "--rm",
                "--entrypoint",
                "/sbin/apk",
                tag,
                "info",
                "--no-cache",
                "--no-network",
                "-v",
            ).splitlines()
        )
        for pin in inputs.runtime_packages:
            name, version = pin.split("=", 1)
            assert f"{name}-{version}" in packages
        assert not any(name.startswith(("python3-", "uv-")) for name in packages)
        # Inspect the actual mount destinations and executable directory. An
        # installed uv environment has files here; empty mount directories alone
        # are not additional runtime packages.
        assert not docker(
            "run", "--rm", "--entrypoint", "/bin/busybox", tag, "find", "/opt", "-type", "f"
        )
        assert (
            docker(
                "run",
                "--rm",
                "--entrypoint",
                "/bin/busybox",
                tag,
                "find",
                "/usr/local/bin",
                "-type",
                "f",
            )
            == "/usr/local/bin/revaer-app"
        )
        uid = docker("run", "--rm", "--entrypoint", "/bin/busybox", tag, "id", "-u")
        gid = docker("run", "--rm", "--entrypoint", "/bin/busybox", tag, "id", "-g")
        assert int(uid) > 0
        assert int(gid) > 0
        for path, permission in (
            ("/usr/local/bin/revaer-app", "555:0:0"),
            ("/app/compliance", "755:0:0"),
            ("/app/compliance/SOURCE-OFFER.txt", "444:0:0"),
            ("/app/docs/api", f"755:{uid}:{gid}"),
            ("/data", f"755:{uid}:{gid}"),
            ("/config", f"755:{uid}:{gid}"),
        ):
            assert (
                docker(
                    "run",
                    "--rm",
                    "--entrypoint",
                    "/bin/busybox",
                    tag,
                    "stat",
                    "-c",
                    "%a:%u:%g",
                    path,
                )
                == permission
            )
        assert selected_builders() == selected
        (tmp_path / "runtime-proof.json").write_text(
            json.dumps(
                {
                    "image_id": image["Id"],
                    "packages": sorted(packages),
                    "uid": uid,
                    "gid": gid,
                    "python_tooling_files_in_runtime": False,
                    "application": "tiny Rust fixture",
                },
                indent=2,
            )
            + "\n"
        )
    finally:
        identifiers = docker("image", "ls", "--quiet", tag).splitlines()
        if identifiers:
            docker("image", "rm", "--force", *identifiers)
        docker("buildx", "rm", "--force", builder)
