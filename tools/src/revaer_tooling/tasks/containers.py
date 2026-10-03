"""Dockerfile orchestration through the installed core Python CLI.

The builder uses uv-managed Python and locked dependencies. The final stage
temporarily mounts that environment through BuildKit, so Python and rv do not
become additional application runtime packages.
"""

from pathlib import Path

from ..bootstrap.selection import native_selection
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.packages import ApkInstallArgs
from ..external.rust import ContainerBuildArgs
from ..images.inputs import BuildInputs, verify_project_pins
from ..images.model import architecture
from .base import Task


def container_inputs(context: Context) -> BuildInputs:
    if context.host.system != "linux" or context.host.uid != 0:
        raise ToolingError("Container preparation requires an Alpine build stage running as root")
    if not context.fs.is_file(Path("/etc/alpine-release")):
        raise ToolingError("Container preparation requires Alpine Linux")
    inputs = BuildInputs.load(context.fs, context.root)
    verify_project_pins(context.fs, context.root, inputs)
    context.tools.python.verify(inputs.python_version)
    return inputs


class ContainerBuild(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        inputs = container_inputs(context)
        settings = context.settings.container
        item = architecture("linux/" + settings.target_arch)
        target = settings.rust_target or item.rust_target
        if (
            settings.builder_image != inputs.builder
            or settings.uv_image != inputs.uv_image
            or settings.rust_version != inputs.rust_version
            or target != item.rust_target
        ):
            raise ToolingError(
                "Docker builder arguments must match the reviewed image/toolchain pins"
            )
        context.tools.uv.verify(inputs.uv_version)
        native = native_selection(context.fs, context.root, (), ())
        context.tools.rustup.install(native.toolchain, native.components)
        context.tools.rustc.verify(inputs.rust_version)
        context.tools.apk.install(ApkInstallArgs(inputs.builder_packages))
        context.tools.rustup.target_add(target)
        output = context.root / "target"
        context.tools.cargo.container_build(ContainerBuildArgs(target, output))
        binary = output / target / "release/revaer-app"
        if binary.is_symlink() or not binary.is_file() or not binary.stat().st_size:
            raise ToolingError("Cargo did not produce the requested container application")
        # Keep the stable COPY path used by the final image, regardless of arch.
        context.fs.copy(binary, output / "release/revaer-app")
        return TaskResult(f"Container application built for {target}")


class ContainerRuntime(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        inputs = container_inputs(context)
        settings = context.settings.container
        if (
            settings.runtime_image != inputs.runtime
            or settings.alpine_version != inputs.alpine_version
            or context.fs.read(Path("/etc/alpine-release")).strip() != inputs.alpine_version
        ):
            raise ToolingError("Docker runtime arguments and installed Alpine must match the pins")
        context.tools.apk.install(ApkInstallArgs(inputs.runtime_packages))
        gid = context.tools.addgroup.ensure_system("revaer")
        account = context.tools.adduser.ensure_system("revaer", "revaer", gid)
        for path, uid, owner_gid in (
            (Path("/app"), 0, 0),
            (Path("/app/compliance"), 0, 0),
            (Path("/data"), account.uid, account.gid),
            (Path("/config"), account.uid, account.gid),
            (Path("/app/docs/api"), account.uid, account.gid),
        ):
            context.fs.mkdir(path)
            context.fs.directory_permissions(path, uid, owner_gid, 0o755)
        return TaskResult("Pinned runtime packages, account and directories prepared")
