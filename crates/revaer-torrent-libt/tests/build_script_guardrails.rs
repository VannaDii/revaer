use std::error::Error;
use std::fs;

#[path = "../build_support.rs"]
mod build_support;

const BUILD_SCRIPT: &str = include_str!("../build.rs");
const SESSION_CPP: &str = include_str!("../src/ffi/session.cpp");

#[test]
fn pkg_config_defines_are_ordered_and_forwarded() {
    let defines = build_support::ordered_defines([
        ("TORRENT_USE_OPENSSL".to_string(), None),
        ("TORRENT_ABI_VERSION".to_string(), Some("2".to_string())),
    ]);

    assert_eq!(
        defines,
        vec![
            ("TORRENT_ABI_VERSION".to_string(), Some("2".to_string())),
            ("TORRENT_USE_OPENSSL".to_string(), None),
        ]
    );
    assert!(BUILD_SCRIPT.contains("ordered_defines(libtorrent.defines)"));
    assert!(BUILD_SCRIPT.contains("bridge.define(&name, value.as_deref())"));
    assert!(BUILD_SCRIPT.contains("REVAER_LIBTORRENT_ABI_VERSION"));
    assert!(BUILD_SCRIPT.contains("bridge.flag(\"-fexceptions\")"));
    assert!(BUILD_SCRIPT.contains("cargo:rerun-if-changed=build_support.rs"));
}

#[test]
fn manual_defines_require_a_single_valid_abi() {
    let defines = build_support::parse_defines(
        "TORRENT_USE_OPENSSL;TORRENT_ABI_VERSION=2;BOOST_SYSTEM_USE_UTF8",
    );
    assert!(
        defines
            .as_ref()
            .is_some_and(|defines| { build_support::abi_version(defines) == Some("2") })
    );
    assert!(build_support::parse_defines("TORRENT_ABI_VERSION=2;TORRENT_ABI_VERSION=1").is_none());
    assert!(build_support::parse_defines("torrent_abi=2").is_none());
    assert!(build_support::parse_defines("TORRENT_ABI_VERSION=").is_none());
    assert_eq!(
        build_support::preprocessor_abi_version(
            "#define OTHER 1\n#define TORRENT_ABI_VERSION 2\n",
        )
        .as_deref(),
        Some("2")
    );
    assert!(build_support::preprocessor_abi_version("#define TORRENT_ABI_VERSION 0\n").is_none());
    assert!(BUILD_SCRIPT.contains("probe_header_abi_version(&bridge)"));
}

#[test]
fn dependency_headers_are_selected_from_the_libtorrent_prefix() -> Result<(), Box<dyn Error>> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("homebrew");
    let libtorrent_include = root.join("Cellar/libtorrent-rasterbar/2.1.0/include");
    let boost_include = root.join("opt/boost/include");
    let openssl_include = root.join("opt/openssl@3/include");
    fs::create_dir_all(libtorrent_include.join("libtorrent"))?;
    fs::create_dir_all(boost_include.join("boost"))?;
    fs::create_dir_all(openssl_include.join("openssl"))?;
    fs::write(boost_include.join("boost/version.hpp"), b"fixture")?;
    fs::write(openssl_include.join("openssl/opensslv.h"), b"fixture")?;

    assert_eq!(
        build_support::dependency_include_path(
            std::slice::from_ref(&libtorrent_include),
            None,
            std::path::Path::new("boost/version.hpp"),
            &["boost"],
        ),
        Some(boost_include)
    );
    assert_eq!(
        build_support::dependency_include_path(
            std::slice::from_ref(&libtorrent_include),
            None,
            std::path::Path::new("openssl/opensslv.h"),
            &["openssl@3", "openssl"],
        ),
        Some(openssl_include)
    );
    Ok(())
}

#[test]
fn native_source_selection_keeps_prefixes_coherent() -> Result<(), Box<dyn Error>> {
    let fixture = tempfile::tempdir()?;
    let headers_only = fixture.path().join("headers-only");
    let complete = fixture.path().join("complete");
    fs::create_dir_all(headers_only.join("include/libtorrent"))?;
    fs::create_dir_all(complete.join("include/libtorrent"))?;
    fs::create_dir_all(complete.join("lib"))?;
    fs::write(complete.join("lib/libtorrent-rasterbar.dylib"), b"fixture")?;

    let selected = build_support::first_complete_prefix(
        [headers_only, complete.clone()],
        |include| include.join("libtorrent").is_dir(),
        |lib| lib.join("libtorrent-rasterbar.dylib").is_file(),
    );

    assert_eq!(
        selected,
        Some((complete.join("include"), complete.join("lib")))
    );
    assert!(!BUILD_SCRIPT.contains("for prefix in [\"/opt/homebrew\", \"/usr/local\"]"));
    assert!(BUILD_SCRIPT.contains("let pkg_config_result"));
    assert!(BUILD_SCRIPT.contains("let Some((include, lib)) = prefix_paths() else"));
    Ok(())
}

#[test]
fn supported_version_window_covers_20_and_21_api_lines() {
    let minimum = (2, 0, 10);
    let maximum_exclusive = (2, 2, 0);

    assert!(!build_support::version_is_supported(
        (2, 0, 9),
        minimum,
        maximum_exclusive
    ));
    assert!(build_support::version_is_supported(
        (2, 0, 10),
        minimum,
        maximum_exclusive
    ));
    assert!(build_support::version_is_supported(
        (2, 1, 0),
        minimum,
        maximum_exclusive
    ));
    assert!(build_support::version_is_supported(
        (2, 1, 99),
        minimum,
        maximum_exclusive
    ));
    assert!(!build_support::version_is_supported(
        (2, 2, 0),
        minimum,
        maximum_exclusive
    ));
    assert!(BUILD_SCRIPT.contains("const MAX_EXCLUSIVE_VERSION: &str = \"2.2.0\""));
}

#[test]
fn macos_architecture_matching_requires_exact_tokens() {
    assert_eq!(build_support::macos_lipo_arch("aarch64"), Some("arm64"));
    assert_eq!(build_support::macos_lipo_arch("x86_64"), Some("x86_64"));
    assert_eq!(build_support::macos_lipo_arch("powerpc"), None);
    assert!(build_support::lipo_output_contains_architecture(
        "Non-fat file: libtorrent-rasterbar.dylib is architecture: arm64",
        "arm64"
    ));
    assert!(!build_support::lipo_output_contains_architecture(
        "Architectures in the fat file are: x86_64 arm64e",
        "arm64"
    ));
    assert!(BUILD_SCRIPT.contains("MissingLinkLibrary"));
}

#[test]
fn native_shim_retains_both_libtorrent_api_paths() {
    for expected in [
        "#if LIBTORRENT_VERSION_NUM >= 20100",
        "#if LIBTORRENT_VERSION_NUM < 20100",
        "lt::load_torrent_buffer",
        "std::make_shared<lt::torrent_info>",
        "lt::create_torrent builder(std::move(create_files)",
        "lt::create_torrent builder(storage",
        "status.need_save_resume_data",
        "status.need_save_resume",
    ] {
        assert!(
            SESSION_CPP.contains(expected),
            "missing API path: {expected}"
        );
    }
}
