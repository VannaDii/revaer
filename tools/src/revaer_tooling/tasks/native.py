"""Produce native analyzer inputs from the checkout's actual Cargo build script."""

import json
import shlex
import tempfile
from itertools import pairwise
from pathlib import Path

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.rust import CargoArgs, CargoOperation
from .base import Task

BRIDGE_HEADERS = (
    "rust/cxx.h",
    "revaer-torrent-libt/src/ffi/bridge.rs.h",
)


def verify_compilation_database(
    document: str, root: Path, required: Path, include_directory: Path | None = None
) -> None:
    """Validate Clang's documented command/arguments alternatives without executing them.

    The build script owns compiler discovery, includes, and defines. rv checks
    that it produced usable inputs for this checkout, including the native bridge.
    A successful Cargo exit alone is insufficient: a cached build may emit no file.
    """
    try:
        entries = json.loads(document)
    except ValueError as error:
        raise ToolingError("Native build produced malformed compilation database JSON") from error
    if not isinstance(entries, list) or not entries:
        raise ToolingError("Native compilation database must contain commands")
    sources: set[Path] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ToolingError("Native compilation database entry must be an object")
        directory, source = entry.get("directory"), entry.get("file")
        if not isinstance(directory, str) or not isinstance(source, str):
            raise ToolingError("Native compile command needs a directory and source file")
        directory_path = Path(directory)
        source_path = (directory_path / source).resolve()
        if (
            not directory_path.is_absolute()
            or not directory_path.is_dir()
            or not source_path.is_relative_to(root.resolve())
            or not source_path.is_file()
        ):
            raise ToolingError("Native compile command does not identify a current checkout file")
        arguments, command = entry.get("arguments"), entry.get("command")
        compiler_arguments: tuple[str, ...]
        if arguments is not None:
            if (
                not isinstance(arguments, list)
                or not arguments
                or not all(isinstance(value, str) and value for value in arguments)
            ):
                raise ToolingError("Native compile arguments must be nonempty strings")
            compiler_arguments = tuple(value for value in arguments if isinstance(value, str))
        elif isinstance(command, str) and command.strip():
            try:
                compiler_arguments = tuple(shlex.split(command))
            except ValueError as error:
                raise ToolingError("Native compile command has invalid quoting") from error
            if not compiler_arguments or not all(compiler_arguments):
                raise ToolingError("Native compile command must contain nonempty arguments")
        else:
            raise ToolingError("Native compile entry has no compiler invocation")
        if include_directory is not None:
            includes = {
                (directory_path / argument[2:]).resolve()
                for argument in compiler_arguments
                if argument.startswith("-I") and len(argument) > 2
            }
            includes.update(
                (directory_path / following).resolve()
                for argument, following in pairwise(compiler_arguments)
                if argument == "-I"
            )
            if include_directory.resolve() not in includes:
                raise ToolingError(
                    "Native compilation database must use the retained bridge headers"
                )
        sources.add(source_path)
    if required.resolve() not in sources:
        raise ToolingError("Native compilation database is missing the libtorrent bridge")
    if include_directory is not None:
        for name in BRIDGE_HEADERS:
            header = include_directory / name
            if header.is_symlink() or not header.is_file() or header.stat().st_size == 0:
                raise ToolingError(f"Native compilation database is missing retained header {name}")


class SonarCompileDatabase(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        output = context.root / "coverage/compile_commands.json"
        if output.parent.is_symlink():
            raise ToolingError("Native coverage output directory must not be a symlink")
        context.fs.mkdir(output.parent)
        with context.fs.lock(output.parent / ".native-compile.lock"):
            context.fs.remove_owned(output, context.root)
            context.fs.remove_owned(output.parent / "cxxbridge", context.root)
            # The output path is a documented rerun-if-env-changed input of the
            # Rust build script. Vary only the filename: the build script stages
            # headers beside it, and those include paths must survive this task.
            # A temporary parent directory would leave the published JSON pointing
            # at deleted headers. Cargo still owns every compiler argument.
            with tempfile.NamedTemporaryFile(
                prefix="native-input-", suffix=".json", dir=output.parent, delete=False
            ) as stream:
                generated = Path(stream.name)
            try:
                context.tools.cargo.execute(
                    CargoArgs(CargoOperation.BUILD, packages=("revaer-torrent-libt",)),
                    env={
                        "REVAER_NATIVE_IT": "1",
                        "CARGO_TARGET_DIR": str(context.root / "target/sonar-build"),
                        "REVAER_NATIVE_COMPILE_COMMANDS_PATH": str(generated),
                    },
                )
                if not generated.is_file() or generated.stat().st_size == 0:
                    raise ToolingError("Native build did not produce its compilation database")
                document = context.fs.read(generated)
                verify_compilation_database(
                    document,
                    context.root,
                    context.root / "crates/revaer-torrent-libt/src/ffi/session.cpp",
                    output.parent / "cxxbridge/include",
                )
                context.fs.write(output, document)
            finally:
                generated.unlink(missing_ok=True)
        return TaskResult("Native compilation database verified: coverage/compile_commands.json")
