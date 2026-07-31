use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MIN_VERSION: &str = "2.0.10";
const MAX_EXCLUSIVE_VERSION: &str = "2.1.0";
const CXXBRIDGE_RUST_HEADER: &str = "rust/cxx.h";
const CXXBRIDGE_CRATE_HEADER: &str = "revaer-torrent-libt/src/ffi/bridge.rs.h";

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), BuildError> {
    println!("cargo:rerun-if-env-changed=LIBTORRENT_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=LIBTORRENT_LIB_DIR");
    println!("cargo:rerun-if-env-changed=LIBTORRENT_BUNDLE_DIR");
    println!("cargo:rerun-if-env-changed=REVAER_NATIVE_IT");
    println!("cargo:rerun-if-env-changed=REVAER_NATIVE_COMPILE_COMMANDS_PATH");
    println!("cargo:rustc-check-cfg=cfg(libtorrent_native)");
    emit_pkg_config_reruns();

    if env::var_os("CARGO_FEATURE_LIBTORRENT").is_none() {
        return Ok(());
    }
    let native_required = env::var_os("REVAER_NATIVE_IT").is_some();
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or(BuildError::MissingManifestDir)?;

    let mut bridge = cxx_build::bridge("src/ffi/bridge.rs");
    bridge.flag_if_supported("-std=c++17");
    bridge.file("src/ffi/session.cpp");

    let include_dir = PathBuf::from("src/ffi/include");
    bridge.include(&include_dir);

    let libs_result = (|| {
        if let Some((include, lib)) = bundled_paths() {
            ensure_header_version(&include)?;
            ensure_macos_link_architecture(
                std::slice::from_ref(&lib),
                &["torrent-rasterbar".to_string()],
            )?;
            bridge.include(&include);
            println!("cargo:rustc-link-search=native={}", lib.display());
            return Ok(vec!["torrent-rasterbar".to_string()]);
        }

        let include_override = env::var_os("LIBTORRENT_INCLUDE_DIR").map(PathBuf::from);
        let lib_dir_override = env::var_os("LIBTORRENT_LIB_DIR").map(PathBuf::from);

        if include_override.is_some() || lib_dir_override.is_some() {
            let include = include_override
                .as_ref()
                .ok_or(BuildError::MissingIncludeDir)?;
            let lib = lib_dir_override.as_ref().ok_or(BuildError::MissingLibDir)?;
            ensure_header_version(include)?;
            ensure_macos_link_architecture(
                std::slice::from_ref(lib),
                &["torrent-rasterbar".to_string()],
            )?;
            bridge.include(include);
            println!("cargo:rustc-link-search=native={}", lib.display());
            return Ok(vec!["torrent-rasterbar".to_string()]);
        }

        let pkg_config_result = pkg_config::Config::new()
            .cargo_metadata(false)
            .atleast_version(MIN_VERSION)
            .probe("libtorrent-rasterbar");
        if let Ok(libtorrent) = pkg_config_result.as_ref() {
            for path in &libtorrent.include_paths {
                if path.join("libtorrent").exists() {
                    ensure_header_version(path)?;
                }
                bridge.include(path);
            }
            let mut defines = libtorrent.defines.iter().collect::<Vec<_>>();
            defines.sort_by_key(|(name, _)| *name);
            for (name, value) in defines {
                bridge.define(name, value.as_deref());
            }
            ensure_macos_link_architecture(&libtorrent.link_paths, &libtorrent.libs)?;
            for lib_path in &libtorrent.link_paths {
                println!("cargo:rustc-link-search=native={}", lib_path.display());
            }
            return Ok(libtorrent.libs.clone());
        }

        Err(BuildError::PkgConfig(match pkg_config_result {
            Ok(_) => return Ok(Vec::new()),
            Err(err) => err,
        }))
    })();

    let libs = match libs_result {
        Ok(libs) => libs,
        Err(err) => {
            if native_required {
                return Err(err);
            }
            emit_reruns();
            return Ok(());
        }
    };

    let compiler = bridge.get_compiler();
    maybe_write_compile_commands(&manifest_dir, compiler.path(), compiler.args())?;
    bridge.compile("revaer-libtorrent");
    println!("cargo:rustc-cfg=libtorrent_native");
    emit_link_libs(libs);
    emit_reruns();
    Ok(())
}

fn maybe_write_compile_commands(
    manifest_dir: &Path,
    compiler_path: &Path,
    compiler_args: &[OsString],
) -> Result<(), BuildError> {
    let Some(output_path) = env::var_os("REVAER_NATIVE_COMPILE_COMMANDS_PATH").map(PathBuf::from)
    else {
        return Ok(());
    };

    let source_path = manifest_dir
        .join("src/ffi/session.cpp")
        .canonicalize()
        .map_err(|source| BuildError::ResolveCompileCommandsPath { source })?;
    let directory = manifest_dir
        .canonicalize()
        .map_err(|source| BuildError::ResolveCompileCommandsPath { source })?;
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or(BuildError::MissingWorkspaceRoot)?;
    let staged_include_dirs = stage_cxxbridge_headers(&output_path, compiler_args)?;
    let output_object = workspace_root.join("target/sonar/session.cpp.o");
    let command = compile_command(
        compiler_path,
        compiler_args,
        &staged_include_dirs,
        &source_path,
        &output_object,
    );
    let contents = format!(
        "[\n  {{\n    \"directory\": \"{}\",\n    \"file\": \"{}\",\n    \"command\": \"{}\"\n  }}\n]\n",
        json_escape(directory.to_string_lossy().as_ref()),
        json_escape(source_path.to_string_lossy().as_ref()),
        json_escape(&command),
    );

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| BuildError::CreateCompileCommandsDir { source })?;
    }
    fs::write(output_path, contents).map_err(|source| BuildError::WriteCompileCommands { source })
}

fn stage_cxxbridge_headers(
    output_path: &Path,
    compiler_args: &[OsString],
) -> Result<Vec<PathBuf>, BuildError> {
    let output_dir = output_path
        .parent()
        .ok_or(BuildError::MissingCompileCommandsDir)?;
    let staged_include_dir = output_dir.join("cxxbridge").join("include");
    for relative_header in [CXXBRIDGE_RUST_HEADER, CXXBRIDGE_CRATE_HEADER] {
        let source_header = cxxbridge_header_source(compiler_args, relative_header)?;
        let staged_header = staged_include_dir.join(relative_header);
        stage_header(&source_header, &staged_header, relative_header)?;
    }
    Ok(vec![staged_include_dir])
}

fn cxxbridge_header_source(
    compiler_args: &[OsString],
    relative_header: &'static str,
) -> Result<PathBuf, BuildError> {
    compiler_include_dirs(compiler_args)
        .into_iter()
        .map(|include_dir| include_dir.join(relative_header))
        .find(|header| header.is_file())
        .ok_or(BuildError::MissingCxxbridgeHeader { relative_header })
}

fn stage_header(
    source_header: &Path,
    staged_header: &Path,
    relative_header: &'static str,
) -> Result<(), BuildError> {
    let staged_header_dir = staged_header
        .parent()
        .ok_or(BuildError::MissingCxxbridgeHeader { relative_header })?;
    fs::create_dir_all(staged_header_dir)
        .map_err(|source| BuildError::CreateCxxbridgeHeaderDir { source })?;
    fs::copy(source_header, staged_header)
        .map_err(|source| BuildError::CopyCxxbridgeHeader { source })?;
    Ok(())
}

fn compiler_include_dirs(compiler_args: &[OsString]) -> Vec<PathBuf> {
    let mut include_dirs = Vec::new();
    let mut expects_include_dir = false;
    for argument in compiler_args {
        let argument_text = argument.to_string_lossy();
        if expects_include_dir {
            include_dirs.push(PathBuf::from(argument_text.as_ref()));
            expects_include_dir = false;
            continue;
        }
        if argument_text == "-I" {
            expects_include_dir = true;
            continue;
        }
        if let Some(include_dir) = argument_text.strip_prefix("-I")
            && !include_dir.is_empty()
        {
            include_dirs.push(PathBuf::from(include_dir));
        }
    }
    include_dirs
}

fn compile_command(
    compiler_path: &Path,
    compiler_args: &[OsString],
    extra_include_dirs: &[PathBuf],
    source_path: &Path,
    output_object: &Path,
) -> String {
    let mut parts = Vec::with_capacity(compiler_args.len() + extra_include_dirs.len() * 2 + 5);
    parts.push(shell_escape(compiler_path.to_string_lossy().as_ref()));
    for argument in compiler_args {
        parts.push(shell_escape(argument.to_string_lossy().as_ref()));
    }
    for include_dir in extra_include_dirs {
        parts.push("-I".to_string());
        parts.push(shell_escape(include_dir.to_string_lossy().as_ref()));
    }
    parts.push("-c".to_string());
    parts.push(shell_escape(source_path.to_string_lossy().as_ref()));
    parts.push("-o".to_string());
    parts.push(shell_escape(output_object.to_string_lossy().as_ref()));
    parts.join(" ")
}

fn shell_escape(value: &str) -> String {
    if value.is_empty()
        || value.chars().any(|character| {
            character.is_whitespace()
                || matches!(character, '"' | '\\' | '$' | '`' | '\'' | '!' | '&' | '|')
        })
    {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn bundled_paths() -> Option<(PathBuf, PathBuf)> {
    let root = env::var_os("LIBTORRENT_BUNDLE_DIR").map(PathBuf::from)?;
    let include = root.join("include");
    let lib = root.join("lib");
    if include.join("libtorrent").exists() && lib.exists() {
        Some((include, lib))
    } else {
        None
    }
}

fn emit_pkg_config_reruns() {
    for variable in [
        "PKG_CONFIG",
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_LIBDIR",
        "PKG_CONFIG_SYSROOT_DIR",
        "PKG_CONFIG_ALLOW_CROSS",
        "PKG_CONFIG_ALLOW_SYSTEM_CFLAGS",
        "PKG_CONFIG_ALLOW_SYSTEM_LIBS",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    if let Some(target) = env::var_os("TARGET").and_then(|value| value.into_string().ok()) {
        let normalized_target = target.replace('-', "_");
        for prefix in [
            "PKG_CONFIG",
            "PKG_CONFIG_PATH",
            "PKG_CONFIG_LIBDIR",
            "PKG_CONFIG_SYSROOT_DIR",
        ] {
            println!("cargo:rerun-if-env-changed={prefix}_{target}");
            println!("cargo:rerun-if-env-changed={prefix}_{normalized_target}");
        }
    }
}

fn emit_link_libs(libs: Vec<String>) {
    for lib in libs {
        println!("cargo:rustc-link-lib={lib}");
    }
}

fn emit_reruns() {
    // Re-run if the bridge or C++ sources change.
    println!("cargo:rerun-if-changed=src/ffi/bridge.rs");
    println!("cargo:rerun-if-changed=src/ffi/include/revaer/session.hpp");
    println!("cargo:rerun-if-changed=src/ffi/session.cpp");
}

fn ensure_header_version(include_dir: &Path) -> Result<(), BuildError> {
    let header = include_dir.join("libtorrent").join("version.hpp");
    let contents =
        fs::read_to_string(&header).map_err(|source| BuildError::ReadHeader { source })?;

    let major =
        parse_define(&contents, "LIBTORRENT_VERSION_MAJOR").ok_or(BuildError::MissingDefine)?;
    let minor =
        parse_define(&contents, "LIBTORRENT_VERSION_MINOR").ok_or(BuildError::MissingDefine)?;
    let patch =
        parse_define(&contents, "LIBTORRENT_VERSION_TINY").ok_or(BuildError::MissingDefine)?;

    let required = parse_version(MIN_VERSION)?;
    if (major, minor, patch) < required {
        return Err(BuildError::VersionTooOld);
    }
    let unsupported = parse_version(MAX_EXCLUSIVE_VERSION)?;
    if (major, minor, patch) >= unsupported {
        return Err(BuildError::VersionTooNew);
    }
    Ok(())
}

fn parse_define(contents: &str, name: &str) -> Option<u32> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("#define") {
            return None;
        }
        let mut parts = trimmed.split_whitespace();
        let _ = parts.next()?;
        let key = parts.next()?;
        let value = parts.next()?;
        if key == name {
            value.parse::<u32>().ok()
        } else {
            None
        }
    })
}

fn parse_version(value: &str) -> Result<(u32, u32, u32), BuildError> {
    let mut parts = value.split('.');
    let major = parse_version_part(parts.next()).ok_or(BuildError::InvalidMinVersion)?;
    let minor = parse_version_part(parts.next()).ok_or(BuildError::InvalidMinVersion)?;
    let patch = parse_version_part(parts.next()).ok_or(BuildError::InvalidMinVersion)?;
    Ok((major, minor, patch))
}

fn parse_version_part(value: Option<&str>) -> Option<u32> {
    value.and_then(|part| part.parse::<u32>().ok())
}

fn ensure_macos_link_architecture(
    link_paths: &[PathBuf],
    libs: &[String],
) -> Result<(), BuildError> {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return Ok(());
    }

    let rust_arch = env::var("CARGO_CFG_TARGET_ARCH").map_err(|_| BuildError::MissingTargetArch)?;
    let expected_arch =
        macos_lipo_arch(&rust_arch).ok_or(BuildError::UnsupportedTargetArch { arch: rust_arch })?;

    for lib in libs {
        if let Some(library_path) = find_link_library(link_paths, lib) {
            ensure_macos_library_contains_architecture(&library_path, expected_arch)?;
        }
    }

    Ok(())
}

fn macos_lipo_arch(rust_arch: &str) -> Option<&'static str> {
    match rust_arch {
        "aarch64" => Some("arm64"),
        "x86_64" => Some("x86_64"),
        _ => None,
    }
}

fn find_link_library(link_paths: &[PathBuf], lib: &str) -> Option<PathBuf> {
    let candidates = [
        format!("lib{lib}.dylib"),
        format!("lib{lib}.a"),
        format!("{lib}.framework/{lib}"),
    ];
    link_paths
        .iter()
        .flat_map(|link_path| {
            candidates
                .iter()
                .map(move |candidate| link_path.join(candidate))
        })
        .find(|candidate| candidate.is_file())
}

fn ensure_macos_library_contains_architecture(
    library_path: &Path,
    expected_arch: &str,
) -> Result<(), BuildError> {
    let output = Command::new("lipo")
        .arg("-info")
        .arg(library_path)
        .output()
        .map_err(|source| BuildError::InspectLibraryArchitecture {
            path: library_path.to_path_buf(),
            source,
        })?;

    let probe_output = lipo_probe_output(&output.stdout, &output.stderr);
    if !output.status.success() {
        return Err(BuildError::LibraryArchitectureProbeFailed {
            path: library_path.to_path_buf(),
            output: probe_output,
        });
    }
    if !lipo_output_contains_architecture(&probe_output, expected_arch) {
        return Err(BuildError::LibraryArchitectureMismatch {
            path: library_path.to_path_buf(),
            expected_arch: expected_arch.to_string(),
            output: probe_output,
        });
    }

    Ok(())
}

fn lipo_probe_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = String::new();
    output.push_str(&String::from_utf8_lossy(stdout));
    output.push_str(&String::from_utf8_lossy(stderr));
    output
}

fn lipo_output_contains_architecture(output: &str, expected_arch: &str) -> bool {
    output
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == expected_arch)
}

#[derive(Debug)]
enum BuildError {
    MissingManifestDir,
    MissingWorkspaceRoot,
    MissingCompileCommandsDir,
    MissingIncludeDir,
    MissingLibDir,
    MissingTargetArch,
    UnsupportedTargetArch {
        arch: String,
    },
    MissingCxxbridgeHeader {
        relative_header: &'static str,
    },
    PkgConfig(pkg_config::Error),
    ReadHeader {
        source: std::io::Error,
    },
    InspectLibraryArchitecture {
        path: PathBuf,
        source: std::io::Error,
    },
    LibraryArchitectureProbeFailed {
        path: PathBuf,
        output: String,
    },
    LibraryArchitectureMismatch {
        path: PathBuf,
        expected_arch: String,
        output: String,
    },
    ResolveCompileCommandsPath {
        source: std::io::Error,
    },
    CreateCompileCommandsDir {
        source: std::io::Error,
    },
    CreateCxxbridgeHeaderDir {
        source: std::io::Error,
    },
    CopyCxxbridgeHeader {
        source: std::io::Error,
    },
    WriteCompileCommands {
        source: std::io::Error,
    },
    MissingDefine,
    InvalidMinVersion,
    VersionTooOld,
    VersionTooNew,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingManifestDir => write!(f, "cargo manifest directory missing"),
            Self::MissingWorkspaceRoot => write!(f, "workspace root unavailable"),
            Self::MissingCompileCommandsDir => write!(f, "compile commands directory unavailable"),
            Self::MissingIncludeDir => write!(f, "libtorrent include directory missing"),
            Self::MissingLibDir => write!(f, "libtorrent library directory missing"),
            Self::MissingTargetArch => write!(f, "cargo target architecture missing"),
            Self::UnsupportedTargetArch { arch } => {
                write!(f, "unsupported macOS target architecture: {arch}")
            }
            Self::MissingCxxbridgeHeader { relative_header } => {
                write!(f, "generated CXX bridge header missing: {relative_header}")
            }
            Self::PkgConfig(_) => write!(f, "libtorrent pkg-config probe failed"),
            Self::ReadHeader { .. } => write!(f, "libtorrent version header read failed"),
            Self::InspectLibraryArchitecture { path, .. } => write!(
                f,
                "library architecture probe failed for {}",
                path.display()
            ),
            Self::LibraryArchitectureProbeFailed { path, output } => write!(
                f,
                "library architecture probe failed for {}: {}",
                path.display(),
                output.trim()
            ),
            Self::LibraryArchitectureMismatch {
                path,
                expected_arch,
                output,
            } => write!(
                f,
                "library architecture mismatch for {}: expected {}, found {}",
                path.display(),
                expected_arch,
                output.trim()
            ),
            Self::ResolveCompileCommandsPath { .. } => {
                write!(f, "compile commands path resolution failed")
            }
            Self::CreateCompileCommandsDir { .. } => {
                write!(f, "compile commands directory creation failed")
            }
            Self::CreateCxxbridgeHeaderDir { .. } => {
                write!(f, "staged CXX bridge header directory creation failed")
            }
            Self::CopyCxxbridgeHeader { .. } => {
                write!(f, "generated CXX bridge header staging failed")
            }
            Self::WriteCompileCommands { .. } => {
                write!(f, "compile commands write failed")
            }
            Self::MissingDefine => write!(f, "libtorrent version header missing field"),
            Self::InvalidMinVersion => write!(f, "invalid libtorrent minimum version"),
            Self::VersionTooOld => write!(f, "libtorrent version is too old"),
            Self::VersionTooNew => write!(f, "libtorrent version is unsupported"),
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PkgConfig(err) => Some(err),
            Self::ReadHeader { source, .. } => Some(source),
            Self::InspectLibraryArchitecture { source, .. } => Some(source),
            Self::ResolveCompileCommandsPath { source, .. } => Some(source),
            Self::CreateCompileCommandsDir { source, .. } => Some(source),
            Self::CreateCxxbridgeHeaderDir { source, .. } => Some(source),
            Self::CopyCxxbridgeHeader { source, .. } => Some(source),
            Self::WriteCompileCommands { source, .. } => Some(source),
            _ => None,
        }
    }
}
