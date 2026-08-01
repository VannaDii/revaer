use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeOutput {
    pub(super) streams: Vec<FfprobeStream>,
    #[serde(default)]
    pub(super) frames: Vec<FfprobeFrame>,
    #[serde(default)]
    pub(super) chapters: Vec<FfprobeChapter>,
    pub(super) format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeFrame {
    pub(super) stream_index: Option<u32>,
    #[serde(default)]
    pub(super) side_data_list: Vec<FfprobeSideData>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeStream {
    pub(super) index: u32,
    pub(super) codec_type: String,
    pub(super) codec_name: Option<String>,
    pub(super) channels: Option<u32>,
    pub(super) channel_layout: Option<String>,
    pub(super) disposition: Option<FfprobeDisposition>,
    pub(super) tags: Option<FfprobeTags>,
    pub(super) profile: Option<String>,
    pub(super) duration: Option<String>,
    pub(super) bit_rate: Option<String>,
    pub(super) sample_rate: Option<String>,
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) pix_fmt: Option<String>,
    pub(super) sample_aspect_ratio: Option<String>,
    pub(super) display_aspect_ratio: Option<String>,
    pub(super) avg_frame_rate: Option<String>,
    pub(super) color_range: Option<String>,
    pub(super) color_space: Option<String>,
    pub(super) color_transfer: Option<String>,
    pub(super) color_primaries: Option<String>,
    pub(super) chroma_location: Option<String>,
    pub(super) field_order: Option<String>,
    #[serde(default)]
    pub(super) side_data_list: Vec<FfprobeSideData>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeDisposition {
    pub(super) default: Option<u8>,
    pub(super) forced: Option<u8>,
    pub(super) hearing_impaired: Option<u8>,
    pub(super) visual_impaired: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeTags {
    pub(super) language: Option<String>,
    pub(super) title: Option<String>,
    #[serde(flatten)]
    pub(super) extra: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeFormat {
    pub(super) format_name: String,
    pub(super) duration: Option<String>,
    pub(super) start_time: Option<String>,
    pub(super) size: Option<String>,
    pub(super) bit_rate: Option<String>,
    #[serde(default)]
    pub(super) tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeChapter {
    pub(super) id: u32,
    pub(super) start_time: String,
    pub(super) end_time: String,
    #[serde(default)]
    pub(super) tags: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeSideData {
    pub(super) side_data_type: String,
    #[serde(flatten)]
    pub(super) extra: BTreeMap<String, Value>,
}
