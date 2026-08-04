use std::collections::BTreeSet;

use super::CodecCapability;

pub(super) fn parse_version_line(output: &str) -> Option<String> {
    output.lines().next().and_then(|line| {
        let mut parts = line.split_whitespace();
        let _program = parts.next()?;
        let _keyword = parts.next()?;
        parts.next().map(str::to_string)
    })
}

pub(super) fn parse_codecs(output: &str) -> Vec<CodecCapability> {
    let codecs = output
        .lines()
        .filter_map(parse_codec_line)
        .collect::<BTreeSet<_>>();
    codecs
        .into_iter()
        .map(
            |(name, encode_supported, decode_supported)| CodecCapability {
                name,
                encode_supported,
                decode_supported,
            },
        )
        .collect()
}

fn parse_codec_line(line: &str) -> Option<(String, bool, bool)> {
    if !line.starts_with(' ') {
        return None;
    }
    let mut tokens = line.split_whitespace();
    let flags = tokens.next()?;
    if flags.len() < 6 {
        return None;
    }
    let codec_name = tokens.next()?;
    let mut chars = flags.chars();
    let decode_supported = chars.next().is_some_and(|item| item == 'D');
    let encode_supported = chars.next().is_some_and(|item| item == 'E');
    Some((codec_name.to_string(), encode_supported, decode_supported))
}

pub(super) fn parse_tool_names(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(parse_tool_name)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parse_tool_name(line: &str) -> Option<String> {
    if !line.starts_with(' ') {
        return None;
    }
    let mut tokens = line.split_whitespace();
    let flags = tokens.next()?;
    if flags.is_empty() {
        return None;
    }
    tokens.next().map(str::to_string)
}

pub(super) fn parse_hwaccels(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.ends_with(':'))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn subtitle_codecs_from(
    codecs: &[CodecCapability],
    encoders: &[String],
    decoders: &[String],
) -> Vec<String> {
    codecs
        .iter()
        .map(|codec| codec.name.as_str())
        .chain(encoders.iter().map(String::as_str))
        .chain(decoders.iter().map(String::as_str))
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .filter(|name| is_subtitle_codec(name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_subtitle_codec(name: &str) -> bool {
    matches!(
        name,
        "ass"
            | "ssa"
            | "srt"
            | "subrip"
            | "webvtt"
            | "mov_text"
            | "hdmv_pgs_subtitle"
            | "dvd_subtitle"
    )
}

pub(super) fn license_mode_from_version_output(output: &str) -> String {
    if output.contains("--enable-nonfree") {
        "nonfree".to_string()
    } else if output.contains("--enable-gpl") {
        "gpl".to_string()
    } else {
        "lgpl".to_string()
    }
}

pub(super) fn absent_capabilities_from(
    hardware_accelerators: &[String],
    encoders: &[String],
) -> Vec<String> {
    let mut absent = BTreeSet::new();
    if hardware_accelerators.is_empty() {
        absent.insert("hardware_acceleration".to_string());
    }
    if !encoders
        .iter()
        .any(|encoder| encoder.eq_ignore_ascii_case("libfdk_aac"))
    {
        absent.insert("libfdk_aac".to_string());
    }
    absent.into_iter().collect()
}

pub(super) fn license_excluded_capabilities(output: &str) -> Vec<String> {
    if output.contains("--enable-nonfree") {
        Vec::new()
    } else {
        [
            "--enable-nonfree",
            "license-incompatible-codecs",
            "proprietary-codecs",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }
}

pub(super) fn filesystem_utilities() -> Vec<String> {
    [
        "atomic_rename",
        "copy",
        "create_dir_all",
        "remove_file",
        "quarantine",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(super) fn utility_capabilities() -> Vec<String> {
    [
        "ffmpeg",
        "ffprobe",
        "ffplay",
        "managed_workspace",
        "final_graph_verify",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(super) fn compliance_links() -> Vec<String> {
    [
        "/app/compliance/SOURCE-OFFER.txt",
        "/app/compliance/THIRD-PARTY-NOTICES.md",
        "/app/compliance/media-runtime-inventory.spdx.json",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
