#![forbid(unsafe_code)]
#![deny(
    warnings,
    dead_code,
    unused,
    unused_imports,
    unused_must_use,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    missing_docs
)]
//! Sync Nexus vendor assets into the Revaer UI static directory.
//!
//! # Design
//! - Resolves the UI root relative to `CARGO_MANIFEST_DIR` so it can be run from any cwd.
//! - Copies CSS and JS into `static/nexus`, replacing previous synced outputs.
//! - Validates required runtime SVGs, UTF-8 text, SVG structure, and canonical URLs.
//! - Rejects raster extensions from the committed `static` runtime asset tree.
//! - Validates the copied CSS for size and a `DaisyUI` marker before writing the lock file.
//! - Emits a deterministic `ASSET_LOCK.txt` containing the CSS hash and directory stats.
//!
//! Failure modes include missing vendor or runtime inputs, copy errors, invalid CSS
//! or JavaScript contents, invalid runtime assets, or inability to write outputs
//! and the lock file.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use fs_extra::dir::CopyOptions;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const VENDOR_ROOT: &str = "ui_vendor/nexus-html@3.1.0";
const STATIC_ROOT: &str = "static";
const OUTPUT_ROOT: &str = "static/nexus";
const MIN_CSS_BYTES: usize = 1024;
const CSS_MARKER: &str = ".btn";
const DATATABLES_JS: &str = "components/datatables.js";
const LEGACY_AVATAR_DIRECTORY: &str = "/images/avatars/";
const LEGACY_AVATAR_EXTENSION: &str = ".png";
const RUNTIME_AVATAR_DIRECTORY: &str = "/static/nexus/images/avatars/";
const RUNTIME_AVATAR_EXTENSION: &str = ".svg";
const DATATABLES_AVATAR_COUNT: u8 = 10;
const DASHBOARD_PRODUCT_COUNT: u8 = 10;
const RUNTIME_ICON_URL: &str = "/static/icons/app-icon.svg";
const RUNTIME_LOGO_URL: &str = "/static/revaer-logo.svg";
const REVAER_BRAND_SVGS: &[&str] = &["icons/app-icon.svg", "revaer-logo.svg"];
const REVAER_BRAND_IDENTIFIERS: &[&str] = &["revaer-purple-gradient", "revaer-r-silhouette"];
const REQUIRED_STATIC_SVGS: &[&str] = &[
    "icons/app-icon.svg",
    "revaer-logo.svg",
    "nexus/images/landing/footer-grainy.svg",
    "nexus/images/landing/hero-bg-gradient.svg",
    "nexus/images/landing/hero-text-underline.svg",
    "nexus/images/landing/showcase-bg-element.svg",
    "nexus/images/landing/showcase-bg-gradient.svg",
    "nexus/images/landing/testimonial-background.svg",
];
const FORBIDDEN_RASTER_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "heic", "heif", "ico", "jpeg", "jpg", "png", "tif", "tiff", "webp",
];
const UTF8_TEXT_EXTENSIONS: &[&str] = &["css", "html", "js", "json", "svg", "txt", "xml"];

/// Errors returned by the asset sync tool.
#[derive(Debug)]
pub enum AssetSyncError {
    /// A required path is missing on disk.
    MissingPath {
        /// Path that could not be found.
        path: PathBuf,
    },
    /// A required file path is not a file.
    ExpectedFile {
        /// Path that was expected to be a file.
        path: PathBuf,
    },
    /// A required directory path is not a directory.
    ExpectedDir {
        /// Path that was expected to be a directory.
        path: PathBuf,
    },
    /// A filesystem operation failed.
    Io {
        /// Path involved in the failing IO operation.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// A directory copy failed.
    CopyFailed {
        /// Copy source path.
        from: PathBuf,
        /// Copy destination path.
        to: PathBuf,
        /// Error message from the copy implementation.
        message: String,
    },
    /// The copied CSS failed the sanity check.
    CssInvalid {
        /// CSS path that failed validation.
        path: PathBuf,
        /// Reason the CSS was rejected.
        reason: String,
    },
    /// The copied JavaScript failed canonicalization.
    JsInvalid {
        /// JavaScript path that failed validation.
        path: PathBuf,
        /// Reason the JavaScript was rejected.
        reason: String,
    },
    /// A committed runtime asset failed validation.
    RuntimeAssetInvalid {
        /// Runtime asset or reference file that failed validation.
        path: PathBuf,
        /// Reason the runtime asset was rejected.
        reason: String,
    },
    /// Traversal of a directory failed.
    WalkFailed {
        /// Directory path that could not be traversed.
        path: PathBuf,
        /// Error message from directory traversal.
        message: String,
    },
}

impl Display for AssetSyncError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPath { path } => {
                write!(formatter, "required path is missing: {}", path.display())
            }
            Self::ExpectedFile { path } => {
                write!(
                    formatter,
                    "expected file but found non-file: {}",
                    path.display()
                )
            }
            Self::ExpectedDir { path } => {
                write!(
                    formatter,
                    "expected directory but found non-directory: {}",
                    path.display()
                )
            }
            Self::Io { path, source } => {
                write!(formatter, "io error at {}: {source}", path.display())
            }
            Self::CopyFailed { from, to, message } => write!(
                formatter,
                "copy failed from {} to {}: {message}",
                from.display(),
                to.display()
            ),
            Self::CssInvalid { path, reason } => write!(
                formatter,
                "copied CSS failed validation at {}: {reason}",
                path.display()
            ),
            Self::JsInvalid { path, reason } => write!(
                formatter,
                "copied JavaScript failed validation at {}: {reason}",
                path.display()
            ),
            Self::RuntimeAssetInvalid { path, reason } => write!(
                formatter,
                "runtime asset failed validation at {}: {reason}",
                path.display()
            ),
            Self::WalkFailed { path, message } => {
                write!(
                    formatter,
                    "directory walk failed at {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl Error for AssetSyncError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DirStats {
    files: u64,
    bytes: u64,
}

/// Run the asset synchronization using the repository-relative paths.
///
/// # Errors
/// Returns an error if vendor or runtime inputs are missing, outputs cannot be
/// written, or copied and committed runtime assets fail validation.
pub fn run() -> Result<(), AssetSyncError> {
    let ui_root = ui_root_dir()?;
    sync_assets(&ui_root)
}

fn ui_root_dir() -> Result<PathBuf, AssetSyncError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| AssetSyncError::MissingPath {
            path: manifest_dir.to_path_buf(),
        })?;
    Ok(ui_root.to_path_buf())
}

fn sync_assets(ui_root: &Path) -> Result<(), AssetSyncError> {
    let vendor_css = ui_root.join(VENDOR_ROOT).join("html/assets/app.css");
    let vendor_js = ui_root.join(VENDOR_ROOT).join("public/js");

    ensure_file(&vendor_css)?;
    ensure_dir(&vendor_js)?;

    let output_root = ui_root.join(OUTPUT_ROOT);
    let output_assets = output_root.join("assets");
    let output_css = output_assets.join("app.css");
    let output_images = output_root.join("images");
    let output_js = output_root.join("js");

    ensure_dir_exists(&output_assets)?;
    ensure_dir(&output_images)?;

    copy_file(&vendor_css, &output_css)?;
    copy_dir(&vendor_js, &output_js)?;
    canonicalize_js_asset_references(&output_js)?;

    validate_css(&output_css)?;
    validate_runtime_assets(ui_root)?;
    validate_runtime_references(ui_root, &output_js)?;

    let css_hash = sha256_hex(&output_css)?;
    let images_stats = dir_stats(&output_images)?;
    let js_stats = dir_stats(&output_js)?;

    write_lock(&output_root, &css_hash, images_stats, js_stats)?;

    Ok(())
}

fn ensure_file(path: &Path) -> Result<(), AssetSyncError> {
    if !path.exists() {
        return Err(AssetSyncError::MissingPath {
            path: path.to_path_buf(),
        });
    }
    if !path.is_file() {
        return Err(AssetSyncError::ExpectedFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn ensure_dir(path: &Path) -> Result<(), AssetSyncError> {
    if !path.exists() {
        return Err(AssetSyncError::MissingPath {
            path: path.to_path_buf(),
        });
    }
    if !path.is_dir() {
        return Err(AssetSyncError::ExpectedDir {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn ensure_dir_exists(path: &Path) -> Result<(), AssetSyncError> {
    fs::create_dir_all(path).map_err(|source| AssetSyncError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn copy_file(from: &Path, to: &Path) -> Result<(), AssetSyncError> {
    if let Some(parent) = to.parent() {
        ensure_dir_exists(parent)?;
    }
    fs::copy(from, to).map_err(|source| AssetSyncError::CopyFailed {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        message: source.to_string(),
    })?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), AssetSyncError> {
    if to.exists() {
        if to.is_dir() {
            fs::remove_dir_all(to).map_err(|source| AssetSyncError::Io {
                path: to.to_path_buf(),
                source,
            })?;
        } else {
            fs::remove_file(to).map_err(|source| AssetSyncError::Io {
                path: to.to_path_buf(),
                source,
            })?;
        }
    }
    let parent = to.parent().ok_or_else(|| AssetSyncError::MissingPath {
        path: to.to_path_buf(),
    })?;
    ensure_dir_exists(parent)?;
    let mut options = CopyOptions::new();
    options.overwrite = true;
    fs_extra::dir::copy(from, parent, &options).map_err(|err| AssetSyncError::CopyFailed {
        from: from.to_path_buf(),
        to: parent.to_path_buf(),
        message: err.to_string(),
    })?;
    Ok(())
}

fn validate_css(path: &Path) -> Result<(), AssetSyncError> {
    let contents = fs::read_to_string(path).map_err(|source| AssetSyncError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if contents.len() < MIN_CSS_BYTES {
        return Err(AssetSyncError::CssInvalid {
            path: path.to_path_buf(),
            reason: format!("expected at least {MIN_CSS_BYTES} bytes"),
        });
    }
    if !contents.contains(CSS_MARKER) {
        return Err(AssetSyncError::CssInvalid {
            path: path.to_path_buf(),
            reason: format!("missing marker {CSS_MARKER}"),
        });
    }
    Ok(())
}

fn canonicalize_js_asset_references(output_js: &Path) -> Result<(), AssetSyncError> {
    let datatables_path = output_js.join(DATATABLES_JS);
    ensure_file(&datatables_path)?;
    let contents = fs::read_to_string(&datatables_path).map_err(|source| AssetSyncError::Io {
        path: datatables_path.clone(),
        source,
    })?;
    let canonical = canonicalize_legacy_avatar_paths(&datatables_path, &contents)?;
    if canonical != contents {
        fs::write(&datatables_path, canonical).map_err(|source| AssetSyncError::Io {
            path: datatables_path,
            source,
        })?;
    }
    Ok(())
}

fn canonicalize_legacy_avatar_paths(path: &Path, contents: &str) -> Result<String, AssetSyncError> {
    let mut canonical = contents.to_string();
    for avatar_index in (1..=DATATABLES_AVATAR_COUNT).rev() {
        let legacy = format!("{LEGACY_AVATAR_DIRECTORY}{avatar_index}{LEGACY_AVATAR_EXTENSION}");
        let runtime_png =
            format!("{RUNTIME_AVATAR_DIRECTORY}{avatar_index}{LEGACY_AVATAR_EXTENSION}");
        let runtime_svg =
            format!("{RUNTIME_AVATAR_DIRECTORY}{avatar_index}{RUNTIME_AVATAR_EXTENSION}");
        canonical = canonical.replace(&legacy, &runtime_svg);
        canonical = canonical.replace(&runtime_png, &runtime_svg);
    }
    if contains_legacy_avatar_url(&canonical) {
        return Err(AssetSyncError::JsInvalid {
            path: path.to_path_buf(),
            reason: "non-canonical DataTables avatar URL".to_string(),
        });
    }
    Ok(canonical)
}

fn contains_legacy_avatar_url(contents: &str) -> bool {
    contents
        .split(RUNTIME_AVATAR_DIRECTORY)
        .any(|segment| segment.contains(LEGACY_AVATAR_DIRECTORY))
}

fn validate_runtime_assets(ui_root: &Path) -> Result<(), AssetSyncError> {
    let static_root = ui_root.join(STATIC_ROOT);
    ensure_dir(&static_root)?;

    for relative_path in REQUIRED_STATIC_SVGS {
        ensure_file(&static_root.join(relative_path))?;
    }
    for asset_index in 1..=DATATABLES_AVATAR_COUNT {
        ensure_file(
            &static_root
                .join("nexus/images/avatars")
                .join(format!("{asset_index}.svg")),
        )?;
    }
    for asset_index in 1..=DASHBOARD_PRODUCT_COUNT {
        ensure_file(
            &static_root
                .join("nexus/images/apps/ecommerce/products")
                .join(format!("{asset_index}.svg")),
        )?;
    }

    for entry in WalkDir::new(&static_root).min_depth(1) {
        let entry = entry.map_err(|err| AssetSyncError::WalkFailed {
            path: static_root.clone(),
            message: err.to_string(),
        })?;
        if entry.file_type().is_file() {
            validate_runtime_asset_file(entry.path())?;
        }
    }
    for relative_path in REVAER_BRAND_SVGS {
        let path = static_root.join(relative_path);
        let contents = read_utf8_text(&path)?;
        validate_revaer_brand_svg(&path, &contents)?;
    }
    Ok(())
}

fn validate_runtime_asset_file(path: &Path) -> Result<(), AssetSyncError> {
    let Some(extension) = path.extension() else {
        return Ok(());
    };
    let extension = extension
        .to_str()
        .ok_or_else(|| AssetSyncError::RuntimeAssetInvalid {
            path: path.to_path_buf(),
            reason: "asset extension is not valid UTF-8".to_string(),
        })?
        .to_ascii_lowercase();

    if FORBIDDEN_RASTER_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AssetSyncError::RuntimeAssetInvalid {
            path: path.to_path_buf(),
            reason: format!("forbidden raster extension .{extension}"),
        });
    }
    if !UTF8_TEXT_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(());
    }

    let contents = read_utf8_text(path)?;
    if extension == "svg" {
        validate_svg_structure(path, &contents)?;
    }
    Ok(())
}

fn read_utf8_text(path: &Path) -> Result<String, AssetSyncError> {
    let bytes = fs::read(path).map_err(|source| AssetSyncError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    String::from_utf8(bytes).map_err(|source| AssetSyncError::RuntimeAssetInvalid {
        path: path.to_path_buf(),
        reason: format!("text is not valid UTF-8: {source}"),
    })
}

fn validate_svg_structure(path: &Path, contents: &str) -> Result<(), AssetSyncError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Err(invalid_runtime_asset(path, "SVG is empty"));
    }
    if contents.contains('\0') {
        return Err(invalid_runtime_asset(path, "SVG contains a NUL byte"));
    }
    let Some(after_root_name) = trimmed.strip_prefix("<svg") else {
        return Err(invalid_runtime_asset(path, "SVG root element is missing"));
    };
    if !after_root_name
        .chars()
        .next()
        .is_some_and(|character| character == '>' || character.is_ascii_whitespace())
    {
        return Err(invalid_runtime_asset(path, "SVG root element is malformed"));
    }
    let opening_end = trimmed
        .find('>')
        .ok_or_else(|| invalid_runtime_asset(path, "SVG root opening tag is incomplete"))?;
    let opening_tag = &trimmed[..=opening_end];
    if opening_tag.ends_with("/>") {
        return Err(invalid_runtime_asset(
            path,
            "SVG root element is self-closing",
        ));
    }
    if !opening_tag.contains("xmlns=\"http://www.w3.org/2000/svg\"")
        && !opening_tag.contains("xmlns='http://www.w3.org/2000/svg'")
    {
        return Err(invalid_runtime_asset(
            path,
            "SVG root is missing the SVG namespace",
        ));
    }
    if !trimmed.ends_with("</svg>") {
        return Err(invalid_runtime_asset(
            path,
            "SVG root closing tag is missing",
        ));
    }
    Ok(())
}

fn invalid_runtime_asset(path: &Path, reason: &str) -> AssetSyncError {
    AssetSyncError::RuntimeAssetInvalid {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

fn validate_revaer_brand_svg(path: &Path, contents: &str) -> Result<(), AssetSyncError> {
    for identifier in REVAER_BRAND_IDENTIFIERS {
        let marker = format!("id=\"{identifier}\"");
        if !contents.contains(&marker) {
            return Err(invalid_runtime_asset(
                path,
                &format!("missing required Revaer brand identifier {identifier}"),
            ));
        }
    }
    Ok(())
}

fn validate_runtime_references(ui_root: &Path, output_js: &Path) -> Result<(), AssetSyncError> {
    validate_reference_count(&ui_root.join("index.html"), RUNTIME_ICON_URL, 3)?;
    validate_reference_count(&ui_root.join("manifest.json"), RUNTIME_ICON_URL, 1)?;
    validate_reference_count(&ui_root.join("browserconfig.xml"), RUNTIME_ICON_URL, 3)?;
    validate_reference_count(
        &ui_root.join("src/components/shell.rs"),
        RUNTIME_LOGO_URL,
        2,
    )?;
    validate_datatables_references(&output_js.join(DATATABLES_JS))
}

fn validate_reference_count(
    path: &Path,
    expected_reference: &str,
    expected_count: usize,
) -> Result<(), AssetSyncError> {
    ensure_file(path)?;
    let contents = read_utf8_text(path)?;
    let actual_count = contents.matches(expected_reference).count();
    if actual_count != expected_count {
        return Err(AssetSyncError::RuntimeAssetInvalid {
            path: path.to_path_buf(),
            reason: format!(
                "expected {expected_count} references to {expected_reference}, found {actual_count}"
            ),
        });
    }
    Ok(())
}

fn validate_datatables_references(path: &Path) -> Result<(), AssetSyncError> {
    ensure_file(path)?;
    let contents = read_utf8_text(path)?;
    if contains_legacy_avatar_url(&contents) {
        return Err(AssetSyncError::JsInvalid {
            path: path.to_path_buf(),
            reason: "non-canonical DataTables avatar URL".to_string(),
        });
    }
    for avatar_index in 1..=DATATABLES_AVATAR_COUNT {
        let expected =
            format!("{RUNTIME_AVATAR_DIRECTORY}{avatar_index}{RUNTIME_AVATAR_EXTENSION}");
        if !contents.contains(&expected) {
            return Err(AssetSyncError::JsInvalid {
                path: path.to_path_buf(),
                reason: format!("missing DataTables avatar URL {expected}"),
            });
        }
    }
    Ok(())
}

fn sha256_hex(path: &Path) -> Result<String, AssetSyncError> {
    let bytes = fs::read(path).map_err(|source| AssetSyncError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn dir_stats(path: &Path) -> Result<DirStats, AssetSyncError> {
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    for entry in WalkDir::new(path).min_depth(1) {
        let entry = entry.map_err(|err| AssetSyncError::WalkFailed {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        if entry.file_type().is_file() {
            let metadata = entry.metadata().map_err(|err| AssetSyncError::WalkFailed {
                path: entry.path().to_path_buf(),
                message: err.to_string(),
            })?;
            files += 1;
            bytes += metadata.len();
        }
    }
    Ok(DirStats { files, bytes })
}

fn write_lock(
    output_root: &Path,
    css_hash: &str,
    images: DirStats,
    js: DirStats,
) -> Result<(), AssetSyncError> {
    ensure_dir_exists(output_root)?;
    let lock_path = output_root.join("ASSET_LOCK.txt");
    let contents = format!(
        "app.css sha256 {css_hash}\nimages files {} bytes {}\njs files {} bytes {}\n",
        images.files, images.bytes, js.files, js.bytes
    );
    fs::write(&lock_path, contents).map_err(|source| AssetSyncError::Io {
        path: lock_path,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt::Write as _;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
    type TestResult = Result<(), Box<dyn Error>>;

    fn repo_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            if ancestor.join("AGENTS.md").is_file() {
                return ancestor.to_path_buf();
            }
        }
        manifest_dir
    }

    fn server_root() -> Result<PathBuf, Box<dyn Error>> {
        let root = repo_root().join(".server_root");
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn css_fixture() -> String {
        let mut css = String::from("/* test */\n.btn { display: inline-flex; }\n");
        let filler = "/* filler */\n";
        while css.len() < MIN_CSS_BYTES {
            css.push_str(filler);
        }
        css
    }

    fn svg_fixture() -> &'static str {
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><path d="M0 0h1v1H0z"/></svg>"#
    }

    fn revaer_brand_svg_fixture() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><linearGradient id="revaer-purple-gradient"><stop stop-color="#d11ac4"/></linearGradient><path id="revaer-r-silhouette" d="M0 0h1v1H0z"/></svg>"##
    }

    fn write_fixture_file(path: &Path, contents: &[u8]) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Result<Self, std::io::Error> {
            let pid = std::process::id();
            loop {
                let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
                let mut path =
                    server_root().map_err(|err| std::io::Error::other(err.to_string()))?;
                path.push(format!("asset-sync-test-{pid}-{counter}"));
                match fs::create_dir(&path) {
                    Ok(()) => return Ok(Self { path }),
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(err) => return Err(err),
                }
            }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_runtime_fixture(root: &Path) -> Result<(), std::io::Error> {
        let static_root = root.join(STATIC_ROOT);
        for relative_path in REQUIRED_STATIC_SVGS {
            write_fixture_file(&static_root.join(relative_path), svg_fixture().as_bytes())?;
        }
        for relative_path in REVAER_BRAND_SVGS {
            write_fixture_file(
                &static_root.join(relative_path),
                revaer_brand_svg_fixture().as_bytes(),
            )?;
        }
        for asset_index in 1..=DATATABLES_AVATAR_COUNT {
            write_fixture_file(
                &static_root
                    .join("nexus/images/avatars")
                    .join(format!("{asset_index}.svg")),
                svg_fixture().as_bytes(),
            )?;
        }
        for asset_index in 1..=DASHBOARD_PRODUCT_COUNT {
            write_fixture_file(
                &static_root
                    .join("nexus/images/apps/ecommerce/products")
                    .join(format!("{asset_index}.svg")),
                svg_fixture().as_bytes(),
            )?;
        }

        write_fixture_file(
            &root.join("index.html"),
            format!("{RUNTIME_ICON_URL}\n{RUNTIME_ICON_URL}\n{RUNTIME_ICON_URL}\n").as_bytes(),
        )?;
        write_fixture_file(
            &root.join("manifest.json"),
            format!("{RUNTIME_ICON_URL}\n").as_bytes(),
        )?;
        write_fixture_file(
            &root.join("browserconfig.xml"),
            format!("{RUNTIME_ICON_URL}\n{RUNTIME_ICON_URL}\n{RUNTIME_ICON_URL}\n").as_bytes(),
        )?;
        write_fixture_file(
            &root.join("src/components/shell.rs"),
            format!("{RUNTIME_LOGO_URL}\n{RUNTIME_LOGO_URL}\n").as_bytes(),
        )
    }

    fn write_vendor_fixture(root: &Path, css: &str) -> Result<(), std::io::Error> {
        write_runtime_fixture(root)?;

        let css_path = root.join(VENDOR_ROOT).join("html/assets/app.css");
        write_fixture_file(&css_path, css.as_bytes())?;

        let js_path = root.join(VENDOR_ROOT).join("public/js");
        fs::create_dir_all(&js_path)?;
        fs::write(js_path.join("app.js"), "console.log('ok');")?;
        fs::create_dir_all(js_path.join("components"))?;
        let mut datatables = String::new();
        for avatar_index in 1..=DATATABLES_AVATAR_COUNT {
            writeln!(
                datatables,
                "const avatar{avatar_index} = \"{LEGACY_AVATAR_DIRECTORY}{avatar_index}{LEGACY_AVATAR_EXTENSION}\";"
            )
            .map_err(|source| std::io::Error::other(source.to_string()))?;
        }
        fs::write(js_path.join(DATATABLES_JS), datatables)?;
        Ok(())
    }

    #[test]
    fn sync_assets_writes_outputs_and_lock() -> TestResult {
        let temp_root = TempRoot::new()?;
        let css_content = css_fixture();
        write_vendor_fixture(&temp_root.path, &css_content)?;

        sync_assets(&temp_root.path)?;

        let output_css = temp_root.path.join(OUTPUT_ROOT).join("assets/app.css");
        let css_contents = fs::read_to_string(&output_css)?;
        assert_eq!(css_contents, css_content);

        let lock_path = temp_root.path.join(OUTPUT_ROOT).join("ASSET_LOCK.txt");
        let lock_contents = fs::read_to_string(&lock_path)?;
        let css_hash = sha256_hex(&output_css)?;
        assert!(lock_contents.contains(&format!("app.css sha256 {css_hash}")));

        let avatar_path = temp_root
            .path
            .join(OUTPUT_ROOT)
            .join("images/avatars/1.svg");
        assert!(avatar_path.is_file());
        assert_eq!(fs::read_to_string(avatar_path)?, svg_fixture());
        let js_dir = temp_root.path.join(OUTPUT_ROOT).join("js");
        assert!(js_dir.join("app.js").is_file());
        let datatables_contents = fs::read_to_string(js_dir.join(DATATABLES_JS))?;
        assert!(datatables_contents.contains("/static/nexus/images/avatars/1.svg"));
        assert!(!contains_legacy_avatar_url(&datatables_contents));
        assert!(!datatables_contents.contains("/static/nexus/images/avatars/1.png"));
        Ok(())
    }

    #[test]
    fn sync_assets_requires_committed_runtime_images() -> TestResult {
        let temp_root = TempRoot::new()?;
        let css_content = css_fixture();
        write_vendor_fixture(&temp_root.path, &css_content)?;
        fs::remove_dir_all(temp_root.path.join(OUTPUT_ROOT).join("images"))?;

        let result = sync_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::MissingPath { .. })),
            "expected MissingPath error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn canonicalize_js_asset_references_requires_datatables_file() -> TestResult {
        let temp_root = TempRoot::new()?;
        let js_dir = temp_root.path.join(OUTPUT_ROOT).join("js");
        fs::create_dir_all(&js_dir)?;

        let result = canonicalize_js_asset_references(&js_dir);
        assert!(
            matches!(result, Err(AssetSyncError::MissingPath { .. })),
            "expected MissingPath error, got {result:?}"
        );

        Ok(())
    }

    #[test]
    fn canonicalize_legacy_avatar_paths_writes_emitted_runtime_url() -> TestResult {
        let canonical = canonicalize_legacy_avatar_paths(
            Path::new("components/datatables.js"),
            r#"const avatar = "/images/avatars/10.png";"#,
        )?;

        assert!(canonical.contains(r"/static/nexus/images/avatars/10.svg"));
        assert!(!contains_legacy_avatar_url(&canonical));
        assert!(!canonical.contains(LEGACY_AVATAR_EXTENSION));
        Ok(())
    }

    #[test]
    fn canonicalize_legacy_avatar_paths_keeps_non_avatar_javascript() -> TestResult {
        let contents = "console.log('ok');";
        let canonical = canonicalize_legacy_avatar_paths(Path::new("app.js"), contents)?;

        assert_eq!(canonical, contents);
        Ok(())
    }

    #[test]
    fn canonicalize_legacy_avatar_paths_rejects_unknown_avatar_png() {
        let result = canonicalize_legacy_avatar_paths(
            Path::new("components/datatables.js"),
            r#"const rows = [{ avatar: "/images/avatars/11.png" }].map((data) => {
    return {
        ...data,
        dateTime: new Date(),
    }
})"#,
        );

        assert!(
            matches!(result, Err(AssetSyncError::JsInvalid { .. })),
            "expected JsInvalid error, got {result:?}"
        );
    }

    #[test]
    fn canonicalize_legacy_avatar_paths_rejects_backtick_unknown_avatar_url() {
        let result = canonicalize_legacy_avatar_paths(
            Path::new("components/datatables.js"),
            r"const avatar = `/images/avatars/${avatarId}.png`;",
        );

        assert!(
            matches!(result, Err(AssetSyncError::JsInvalid { .. })),
            "expected JsInvalid error, got {result:?}"
        );
    }

    #[test]
    fn canonicalize_legacy_avatar_paths_rejects_wrong_svg_url() {
        let result = canonicalize_legacy_avatar_paths(
            Path::new("components/datatables.js"),
            r#"const avatar = "/images/avatars/1.svg";"#,
        );

        assert!(
            matches!(result, Err(AssetSyncError::JsInvalid { .. })),
            "expected JsInvalid error, got {result:?}"
        );
    }

    #[test]
    fn runtime_asset_validation_rejects_missing_required_svg() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        fs::remove_file(temp_root.path.join("static/icons/app-icon.svg"))?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::MissingPath { .. })),
            "expected MissingPath error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_asset_validation_rejects_malformed_svg() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        fs::write(
            temp_root.path.join("static/icons/app-icon.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg"><path/></svg"#,
        )?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_asset_validation_rejects_missing_brand_gradient_identifier() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        let invalid = revaer_brand_svg_fixture()
            .replace("revaer-purple-gradient", "unapproved-purple-gradient");
        fs::write(temp_root.path.join("static/revaer-logo.svg"), invalid)?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_asset_validation_rejects_missing_brand_silhouette_identifier() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        let invalid = revaer_brand_svg_fixture()
            .replace("revaer-r-silhouette", "unapproved-brand-silhouette");
        fs::write(temp_root.path.join("static/icons/app-icon.svg"), invalid)?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_asset_validation_rejects_non_utf8_text() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        fs::write(
            temp_root.path.join("static/icons/app-icon.svg"),
            [0xff, 0xfe, 0xfd],
        )?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_asset_validation_rejects_forbidden_binary_extension() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        fs::write(
            temp_root.path.join("static/nexus/images/legacy.PNG"),
            b"raster",
        )?;

        let result = validate_runtime_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn runtime_reference_validation_rejects_wrong_icon_url() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_runtime_fixture(&temp_root.path)?;
        fs::write(
            temp_root.path.join("index.html"),
            "/icons/app-icon.svg\n/icons/app-icon.svg\n/icons/app-icon.svg\n",
        )?;

        let output_js = temp_root.path.join(OUTPUT_ROOT).join("js");
        let result = validate_runtime_references(&temp_root.path, &output_js);
        assert!(
            matches!(result, Err(AssetSyncError::RuntimeAssetInvalid { .. })),
            "expected RuntimeAssetInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn invalid_css_fails_validation() -> TestResult {
        let temp_root = TempRoot::new()?;
        write_vendor_fixture(&temp_root.path, "body { color: black; }")?;

        let result = sync_assets(&temp_root.path);
        assert!(
            matches!(result, Err(AssetSyncError::CssInvalid { .. })),
            "expected CssInvalid error, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn asset_sync_error_display_and_source_match_variant() {
        let missing_variant = AssetSyncError::MissingPath {
            path: PathBuf::from("static/nexus/missing.css"),
        };
        assert!(
            missing_variant
                .to_string()
                .contains("required path is missing")
        );
        assert!(missing_variant.source().is_none());

        let expected_file_variant = AssetSyncError::ExpectedFile {
            path: PathBuf::from("static/nexus/images"),
        };
        assert!(
            expected_file_variant
                .to_string()
                .contains("expected file but found non-file")
        );
        assert!(expected_file_variant.source().is_none());

        let expected_dir_variant = AssetSyncError::ExpectedDir {
            path: PathBuf::from("static/nexus/assets/app.css"),
        };
        assert!(
            expected_dir_variant
                .to_string()
                .contains("expected directory but found non-directory")
        );
        assert!(expected_dir_variant.source().is_none());

        let io_error = std::io::Error::other("disk full");
        let io_variant = AssetSyncError::Io {
            path: PathBuf::from("static/nexus/app.css"),
            source: io_error,
        };
        assert!(io_variant.to_string().contains("io error at"));
        assert!(io_variant.source().is_some());

        let copy_variant = AssetSyncError::CopyFailed {
            from: PathBuf::from("ui_vendor/app.css"),
            to: PathBuf::from("static/nexus/assets"),
            message: "permission denied".to_string(),
        };
        assert!(copy_variant.to_string().contains("copy failed from"));
        assert!(copy_variant.source().is_none());

        let css_variant = AssetSyncError::CssInvalid {
            path: PathBuf::from("static/nexus/app.css"),
            reason: "missing marker".to_string(),
        };
        assert!(
            css_variant
                .to_string()
                .contains("copied CSS failed validation")
        );
        assert!(css_variant.source().is_none());

        let js_variant = AssetSyncError::JsInvalid {
            path: PathBuf::from("static/nexus/js/components/datatables.js"),
            reason: "missing map".to_string(),
        };
        assert!(
            js_variant
                .to_string()
                .contains("copied JavaScript failed validation")
        );
        assert!(js_variant.source().is_none());

        let runtime_asset_variant = AssetSyncError::RuntimeAssetInvalid {
            path: PathBuf::from("static/icons/app-icon.svg"),
            reason: "malformed SVG".to_string(),
        };
        assert!(
            runtime_asset_variant
                .to_string()
                .contains("runtime asset failed validation")
        );
        assert!(runtime_asset_variant.source().is_none());

        let walk_variant = AssetSyncError::WalkFailed {
            path: PathBuf::from("static/nexus/images"),
            message: "not readable".to_string(),
        };
        assert!(
            walk_variant
                .to_string()
                .contains("directory walk failed at")
        );
        assert!(walk_variant.source().is_none());
    }

    #[test]
    fn ui_root_dir_resolves_to_revaer_ui_crate() -> TestResult {
        let ui_root = ui_root_dir()?;
        assert!(ui_root.ends_with("crates/revaer-ui"));
        Ok(())
    }

    #[test]
    fn ensure_file_errors_for_missing_and_non_file() {
        let temp_root = TempRoot::new().expect("temp root");
        let missing = temp_root.path.join("missing.css");
        let err = ensure_file(&missing).expect_err("expected missing path error");
        assert!(matches!(err, AssetSyncError::MissingPath { .. }));

        let dir_path = temp_root.path.join("dir");
        fs::create_dir_all(&dir_path).expect("create dir");
        let err = ensure_file(&dir_path).expect_err("expected non-file error");
        assert!(matches!(err, AssetSyncError::ExpectedFile { .. }));
    }

    #[test]
    fn ensure_dir_errors_for_missing_and_non_dir() {
        let temp_root = TempRoot::new().expect("temp root");
        let missing = temp_root.path.join("missing");
        let err = ensure_dir(&missing).expect_err("expected missing path error");
        assert!(matches!(err, AssetSyncError::MissingPath { .. }));

        let file_path = temp_root.path.join("file.txt");
        fs::write(&file_path, "data").expect("write file");
        let err = ensure_dir(&file_path).expect_err("expected non-dir error");
        assert!(matches!(err, AssetSyncError::ExpectedDir { .. }));
    }

    #[test]
    fn copy_file_creates_parent_directories_and_preserves_contents() -> TestResult {
        let temp_root = TempRoot::new()?;
        let source = temp_root.path.join("vendor/app.css");
        if let Some(parent) = source.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&source, css_fixture())?;

        let destination = temp_root.path.join("out/assets/app.css");
        copy_file(&source, &destination)?;

        assert_eq!(fs::read_to_string(destination)?, css_fixture());
        Ok(())
    }

    #[test]
    fn copy_file_reports_missing_source() {
        let temp_root = TempRoot::new().expect("temp root");
        let missing = temp_root.path.join("missing.css");
        let destination = temp_root.path.join("dest/app.css");
        let err = copy_file(&missing, &destination).expect_err("expected copy failure");
        assert!(matches!(err, AssetSyncError::CopyFailed { .. }));
    }

    #[test]
    fn copy_dir_replaces_existing_file_target() -> TestResult {
        let temp_root = TempRoot::new()?;
        let source = temp_root.path.join("source-dir");
        fs::create_dir_all(&source)?;
        fs::write(source.join("asset.txt"), "asset")?;

        let destination_parent = temp_root.path.join("dest");
        fs::create_dir_all(&destination_parent)?;
        let destination = destination_parent.join("source-dir");
        fs::write(&destination, "placeholder")?;

        copy_dir(&source, &destination)?;

        let copied_file = destination.join("asset.txt");
        assert!(copied_file.is_file());
        Ok(())
    }

    #[test]
    fn copy_dir_replaces_existing_directory_target() -> TestResult {
        let temp_root = TempRoot::new()?;
        let source = temp_root.path.join("source-dir");
        fs::create_dir_all(&source)?;
        fs::write(source.join("asset.txt"), "asset")?;

        let destination_parent = temp_root.path.join("dest");
        let destination = destination_parent.join("source-dir");
        fs::create_dir_all(&destination)?;
        fs::write(destination.join("stale.txt"), "stale")?;

        copy_dir(&source, &destination)?;

        assert!(!destination.join("stale.txt").exists());
        assert_eq!(fs::read_to_string(destination.join("asset.txt"))?, "asset");
        Ok(())
    }

    #[test]
    fn copy_dir_reports_missing_source_directory() -> TestResult {
        let temp_root = TempRoot::new()?;
        let source = temp_root.path.join("missing-dir");
        let destination = temp_root.path.join("dest/missing-dir");
        let err = copy_dir(&source, &destination).expect_err("expected copy failure");
        assert!(matches!(err, AssetSyncError::CopyFailed { .. }));
        Ok(())
    }

    #[test]
    fn validate_css_flags_short_or_missing_marker() -> TestResult {
        let temp_root = TempRoot::new()?;
        let short_css = temp_root.path.join("short.css");
        fs::write(&short_css, ".btn{}")?;
        let err = validate_css(&short_css).expect_err("expected short css error");
        assert!(matches!(err, AssetSyncError::CssInvalid { .. }));

        let marker_missing = temp_root.path.join("no-marker.css");
        let mut contents = String::from("body { color: black; }\n");
        while contents.len() < MIN_CSS_BYTES {
            contents.push_str("/* filler */\n");
        }
        fs::write(&marker_missing, contents)?;
        let err = validate_css(&marker_missing).expect_err("expected missing marker error");
        assert!(matches!(err, AssetSyncError::CssInvalid { .. }));
        Ok(())
    }

    #[test]
    fn validate_css_accepts_fixture_content() -> TestResult {
        let temp_root = TempRoot::new()?;
        let css_path = temp_root.path.join("valid.css");
        fs::write(&css_path, css_fixture())?;
        validate_css(&css_path)?;
        Ok(())
    }

    #[test]
    fn validate_css_reports_io_for_missing_file() {
        let missing = PathBuf::from("missing.css");
        let err = validate_css(&missing).expect_err("expected io error");
        assert!(matches!(err, AssetSyncError::Io { .. }));
    }

    #[test]
    fn sha256_hex_reports_missing_path() {
        let missing = PathBuf::from("no-such-file");
        let err = sha256_hex(&missing).expect_err("expected hash error");
        assert!(matches!(err, AssetSyncError::Io { .. }));
    }

    #[test]
    fn dir_stats_counts_files_and_reports_errors() -> TestResult {
        let temp_root = TempRoot::new()?;
        let dir = temp_root.path.join("stats");
        fs::create_dir_all(dir.join("nested"))?;
        fs::write(dir.join("a.txt"), "a")?;
        fs::write(dir.join("nested/b.txt"), "b")?;

        let stats = dir_stats(&dir)?;
        assert_eq!(stats.files, 2);
        assert!(stats.bytes >= 2);

        let missing = temp_root.path.join("missing");
        let err = dir_stats(&missing).expect_err("expected walk error");
        assert!(matches!(err, AssetSyncError::WalkFailed { .. }));
        Ok(())
    }

    #[test]
    fn write_lock_reports_io_error_when_output_is_file() -> TestResult {
        let temp_root = TempRoot::new()?;
        let output_root = temp_root.path.join("output");
        fs::write(&output_root, "not a dir")?;

        let err = write_lock(
            &output_root,
            "hash",
            DirStats { files: 0, bytes: 0 },
            DirStats { files: 0, bytes: 0 },
        )
        .expect_err("expected io error");
        assert!(matches!(err, AssetSyncError::Io { .. }));
        Ok(())
    }

    #[test]
    fn write_lock_records_hash_and_directory_stats() -> TestResult {
        let temp_root = TempRoot::new()?;
        let output_root = temp_root.path.join("output");
        write_lock(
            &output_root,
            "abc123",
            DirStats {
                files: 2,
                bytes: 20,
            },
            DirStats {
                files: 1,
                bytes: 10,
            },
        )?;
        let contents = fs::read_to_string(output_root.join("ASSET_LOCK.txt"))?;
        assert!(contents.contains("app.css sha256 abc123"));
        assert!(contents.contains("images files 2 bytes 20"));
        assert!(contents.contains("js files 1 bytes 10"));
        Ok(())
    }

    #[test]
    fn write_lock_reports_io_error_when_lock_path_is_directory() -> TestResult {
        let temp_root = TempRoot::new()?;
        let output_root = temp_root.path.join("output");
        fs::create_dir_all(output_root.join("ASSET_LOCK.txt"))?;

        let err = write_lock(
            &output_root,
            "hash",
            DirStats { files: 1, bytes: 1 },
            DirStats { files: 1, bytes: 1 },
        )
        .expect_err("expected lock write error");
        assert!(matches!(err, AssetSyncError::Io { .. }));
        Ok(())
    }
}
