use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) fn ordered_defines<I>(defines: I) -> Vec<(String, Option<String>)>
where
    I: IntoIterator<Item = (String, Option<String>)>,
{
    let mut defines: Vec<_> = defines.into_iter().collect();
    defines.sort_by(|left, right| left.0.cmp(&right.0));
    defines
}

pub(crate) fn parse_defines(value: &str) -> Option<Vec<(String, Option<String>)>> {
    let mut defines = BTreeMap::new();
    for entry in value.split(';') {
        let (name, value) = match entry.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (entry, None),
        };
        if name.is_empty()
            || name.chars().enumerate().any(|(index, character)| {
                !(character.is_ascii_uppercase()
                    || character == '_'
                    || (index > 0 && character.is_ascii_digit()))
            })
            || value.as_ref().is_some_and(|value| {
                value.is_empty()
                    || !value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
            })
            || defines.insert(name.to_string(), value).is_some()
        {
            return None;
        }
    }
    (!defines.is_empty()).then(|| defines.into_iter().collect())
}

pub(crate) fn abi_version(defines: &[(String, Option<String>)]) -> Option<&str> {
    defines.iter().find_map(|(name, value)| {
        (name == "TORRENT_ABI_VERSION")
            .then_some(value.as_deref())
            .flatten()
            .filter(|value| valid_abi_version(value))
    })
}

pub(crate) fn preprocessor_abi_version(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let directive = fields.next()?;
        let name = fields.next()?;
        let value = fields.next()?;
        (directive == "#define"
            && name == "TORRENT_ABI_VERSION"
            && fields.next().is_none()
            && valid_abi_version(value))
        .then(|| value.to_string())
    })
}

fn valid_abi_version(value: &str) -> bool {
    value
        .parse::<u32>()
        .is_ok_and(|value| (1..=4).contains(&value))
}

pub(crate) fn dependency_include_path(
    libtorrent_include_paths: &[PathBuf],
    override_path: Option<PathBuf>,
    header: &Path,
    opt_names: &[&str],
) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return path.join(header).is_file().then_some(path);
    }

    libtorrent_include_paths.iter().find_map(|include_path| {
        dependency_include_path_from_prefix(include_path, header, opt_names)
    })
}

fn dependency_include_path_from_prefix(
    include_path: &Path,
    header: &Path,
    opt_names: &[&str],
) -> Option<PathBuf> {
    if include_path.join(header).is_file() {
        return Some(include_path.to_path_buf());
    }

    include_path.ancestors().find_map(|ancestor| {
        opt_names
            .iter()
            .map(|opt_name| ancestor.join("opt").join(opt_name).join("include"))
            .chain(std::iter::once(ancestor.join("include")))
            .find(|candidate| candidate.join(header).is_file())
    })
}

pub(crate) fn first_complete_prefix<I, H, L>(
    roots: I,
    has_headers: H,
    has_library: L,
) -> Option<(PathBuf, PathBuf)>
where
    I: IntoIterator<Item = PathBuf>,
    H: Fn(&Path) -> bool,
    L: Fn(&Path) -> bool,
{
    roots
        .into_iter()
        .map(|root| (root.join("include"), root.join("lib")))
        .find(|(include, lib)| has_headers(include) && has_library(lib))
}

pub(crate) fn version_is_supported(
    version: (u32, u32, u32),
    minimum: (u32, u32, u32),
    maximum_exclusive: (u32, u32, u32),
) -> bool {
    version >= minimum && version < maximum_exclusive
}

pub(crate) fn macos_lipo_arch(rust_arch: &str) -> Option<&'static str> {
    match rust_arch {
        "aarch64" => Some("arm64"),
        "x86_64" => Some("x86_64"),
        _ => None,
    }
}

pub(crate) fn lipo_output_contains_architecture(output: &str, expected_arch: &str) -> bool {
    output
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == expected_arch)
}
