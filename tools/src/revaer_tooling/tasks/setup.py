"""Environment setup and diagnostics; only setup installs external tools."""

from ..automation.files import write_values
from ..bootstrap.install import install_kcov
from ..bootstrap.selection import apt_selection, native_selection
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.packages import AptInstallArgs
from ..sonar.install import install_scanner
from .base import Task


class Setup(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        options = context.options
        packages = apt_selection(options.apt_profile, options.apt_packages)
        if packages and context.host.system != "linux":
            raise ToolingError("Apt setup requires a Debian/Ubuntu Linux host")
        if options.profile == "python" and (options.cargo_tools or options.browsers):
            raise ToolingError("Cargo/browser selections require the dev or ci setup profile")
        native = (
            native_selection(context.fs, context.root, options.cargo_tools, options.browsers)
            if options.profile != "python"
            else None
        )
        context.tools.uv.verify(context.fs.read(context.root / ".uv-version").strip())
        context.tools.uv.check_lock()
        if packages:
            context.tools.apt.install(AptInstallArgs(packages))
            if "sccache" in packages and context.settings.workflow.environment is not None:
                context.tools.sccache.verify()
                cache = context.host.home / ".cache/sccache"
                context.fs.mkdir(cache)
                write_values(
                    context.fs,
                    context.settings.workflow.environment,
                    {
                        "RUSTC_WRAPPER": "sccache",
                        "SCCACHE_DIR": str(cache),
                        "SCCACHE_CACHE_SIZE": "5G",
                    },
                )
        if context.options.kcov:
            context.emit("Kcov installed: " + str(install_kcov(context)))
        if context.options.sonar_scanner:
            context.emit("Sonar scanner installed: " + str(install_scanner(context)))
        if context.options.install_launcher:
            context.tools.uv.install_launcher(
                context.root / "tools/launcher",
                context.fs.read(context.root / ".python-version").strip(),
            )
            context.emit("Installed rv with uv; run `uv tool update-shell` if it is not on PATH")
        if native is None:
            return TaskResult("Python environment ready")
        context.tools.rustup.install(native.toolchain, native.components)
        if native.wasm:
            context.tools.rustup.target_add("wasm32-unknown-unknown")
        if native.nightly:
            context.tools.rustup.install(native.nightly, ())
        for package in native.cargo:
            context.tools.cargo.install(package)
        if native.browsers:
            context.tools.python.install_browsers(native.browsers, context.options.profile == "ci")
        # A selective CI install must not require unrelated chart, image or docs
        # tools. The installed tools are checked by their consuming tasks as well.
        if options.cargo_tools is not None or options.browsers is not None:
            return TaskResult("Selected development tools ready")
        return Doctor.run(context)


class Doctor(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        failures: list[str] = []
        tools = (
            context.tools.uv,
            context.tools.cargo,
            context.tools.rustup,
            context.tools.git,
            context.tools.github,
            context.tools.trunk,
            context.tools.mdbook,
            context.tools.mdbook_mermaid,
            context.tools.lychee,
            context.tools.cargo_audit,
            context.tools.cargo_deny,
            context.tools.cargo_llvm_cov,
            context.tools.cargo_udeps,
            context.tools.sqlx,
            context.tools.docker,
            context.tools.buildx,
            context.tools.trivy,
            context.tools.psql,
            context.tools.pg_isready,
            context.tools.listeners,
            context.tools.helm,
            context.tools.oras,
            context.tools.gpg,
            *context.tools.native_prerequisites,
        )
        for tool in tools:
            try:
                result = tool.verify()
                context.emit(f"{tool.name}: {result.version} ({result.executable})")
            except ToolingError as error:
                failures.append(str(error))
        if failures:
            raise ToolingError("Prerequisites incomplete:\n" + "\n".join(failures))
        return TaskResult("Development tools ready")
