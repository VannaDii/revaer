use std::path::{Path, PathBuf};

pub(crate) fn ordered_defines<I>(defines: I) -> Vec<(String, Option<String>)>
where
    I: IntoIterator<Item = (String, Option<String>)>,
{
    let mut defines: Vec<_> = defines.into_iter().collect();
    defines.sort_by(|left, right| left.0.cmp(&right.0));
    defines
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
