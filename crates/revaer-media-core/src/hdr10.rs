//! HDR10 color-volume parsing and geometry helpers.

const CHROMATICITY_EPSILON: f64 = 1.0e-9;

/// Parsed two-dimensional chromaticity coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromaticityPoint {
    x: f64,
    y: f64,
}

impl ChromaticityPoint {
    /// Builds a point only when the coordinates are physically meaningful.
    #[must_use]
    pub fn new_physical(x: f64, y: f64) -> Option<Self> {
        (x > 0.0 && y > 0.0 && x + y <= 1.0).then_some(Self { x, y })
    }
}

/// Borrowed exact HDR10 color-volume field set.
#[derive(Debug, Clone, Copy)]
pub struct Hdr10ColorVolumeParts<'a> {
    /// Red primary x chromaticity coordinate.
    pub mastering_red_x: &'a str,
    /// Red primary y chromaticity coordinate.
    pub mastering_red_y: &'a str,
    /// Green primary x chromaticity coordinate.
    pub mastering_green_x: &'a str,
    /// Green primary y chromaticity coordinate.
    pub mastering_green_y: &'a str,
    /// Blue primary x chromaticity coordinate.
    pub mastering_blue_x: &'a str,
    /// Blue primary y chromaticity coordinate.
    pub mastering_blue_y: &'a str,
    /// White point x chromaticity coordinate.
    pub mastering_white_point_x: &'a str,
    /// White point y chromaticity coordinate.
    pub mastering_white_point_y: &'a str,
    /// Minimum mastering-display luminance.
    pub mastering_min_luminance: &'a str,
    /// Maximum mastering-display luminance.
    pub mastering_max_luminance: &'a str,
    /// Maximum content light level.
    pub max_content_light_level: &'a str,
    /// Maximum frame-average light level.
    pub max_frame_average_light_level: &'a str,
}

/// HDR10 color-volume validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hdr10ColorVolumeValidationError {
    /// Field or field group that failed validation.
    pub field: &'static str,
    /// Offending value or synthetic range marker.
    pub value: String,
}

/// Parses an HDR10 decimal or rational numeric field.
#[must_use]
pub fn parse_number(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let parsed = if let Some((numerator, denominator)) = value.split_once('/') {
        if denominator.contains('/') {
            return None;
        }
        let numerator = numerator.trim().parse::<f64>().ok()?;
        let denominator = denominator.trim().parse::<f64>().ok()?;
        if denominator <= 0.0 {
            return None;
        }
        numerator / denominator
    } else {
        value.parse::<f64>().ok()?
    };
    parsed.is_finite().then_some(parsed)
}

/// Returns whether exact HDR10 color-volume fields are physically valid.
#[must_use]
pub fn color_volume_is_valid(volume: Hdr10ColorVolumeParts<'_>) -> bool {
    validate_color_volume(volume).is_ok()
}

/// Validates exact HDR10 mastering-display and content-light fields.
///
/// # Errors
///
/// Returns [`Hdr10ColorVolumeValidationError`] when a numeric field is invalid,
/// a chromaticity point is nonphysical, the primary triangle is degenerate, the
/// white point is outside the primaries, or luminance/content-light ranges are
/// internally inconsistent.
pub fn validate_color_volume(
    volume: Hdr10ColorVolumeParts<'_>,
) -> Result<(), Hdr10ColorVolumeValidationError> {
    let red = chromaticity_point(
        "mastering_red",
        volume.mastering_red_x,
        volume.mastering_red_y,
    )?;
    let green = chromaticity_point(
        "mastering_green",
        volume.mastering_green_x,
        volume.mastering_green_y,
    )?;
    let blue = chromaticity_point(
        "mastering_blue",
        volume.mastering_blue_x,
        volume.mastering_blue_y,
    )?;
    let white_point = chromaticity_point(
        "mastering_white_point",
        volume.mastering_white_point_x,
        volume.mastering_white_point_y,
    )?;
    if !mastering_primaries_are_non_degenerate(red, green, blue) {
        return Err(invalid_color_volume("mastering_primaries", "degenerate"));
    }
    if !chromaticity_point_inside_mastering_primaries(white_point, red, green, blue) {
        return Err(invalid_color_volume(
            "mastering_white_point",
            "outside_primaries",
        ));
    }
    let min_luminance = number("mastering_min_luminance", volume.mastering_min_luminance)?;
    let max_luminance = number("mastering_max_luminance", volume.mastering_max_luminance)?;
    if min_luminance < 0.0 || max_luminance <= min_luminance {
        return Err(invalid_color_volume("mastering_luminance", "invalid_range"));
    }
    let max_content = positive_number("max_content_light_level", volume.max_content_light_level)?;
    let max_average = positive_number(
        "max_frame_average_light_level",
        volume.max_frame_average_light_level,
    )?;
    if max_average > max_content {
        return Err(invalid_color_volume(
            "max_frame_average_light_level",
            volume.max_frame_average_light_level,
        ));
    }
    Ok(())
}

/// Returns whether the mastering-primary triangle has usable area.
#[must_use]
pub fn mastering_primaries_are_non_degenerate(
    red: ChromaticityPoint,
    green: ChromaticityPoint,
    blue: ChromaticityPoint,
) -> bool {
    chromaticity_triangle_area(red, green, blue) > CHROMATICITY_EPSILON
}

/// Returns whether a point is inside the mastering-primary triangle.
#[must_use]
pub fn chromaticity_point_inside_mastering_primaries(
    point: ChromaticityPoint,
    red: ChromaticityPoint,
    green: ChromaticityPoint,
    blue: ChromaticityPoint,
) -> bool {
    let denominator =
        (blue.x - green.x).mul_add(red.y - blue.y, (green.y - blue.y) * (red.x - blue.x));
    if denominator.abs() <= CHROMATICITY_EPSILON {
        return false;
    }
    let alpha = (blue.x - green.x)
        .mul_add(point.y - blue.y, (green.y - blue.y) * (point.x - blue.x))
        / denominator;
    let beta = (red.x - blue.x).mul_add(point.y - blue.y, (blue.y - red.y) * (point.x - blue.x))
        / denominator;
    let gamma = 1.0 - alpha - beta;
    let barycentric_range = -CHROMATICITY_EPSILON..=1.0 + CHROMATICITY_EPSILON;
    [alpha, beta, gamma]
        .into_iter()
        .all(|value| barycentric_range.contains(&value))
}

/// Returns whether mastering-display coordinates form a valid color volume.
#[must_use]
pub fn mastering_display_chromaticities_are_valid(
    red: ChromaticityPoint,
    green: ChromaticityPoint,
    blue: ChromaticityPoint,
    white_point: ChromaticityPoint,
) -> bool {
    mastering_primaries_are_non_degenerate(red, green, blue)
        && chromaticity_point_inside_mastering_primaries(white_point, red, green, blue)
}

fn chromaticity_point(
    prefix: &'static str,
    x: &str,
    y: &str,
) -> Result<ChromaticityPoint, Hdr10ColorVolumeValidationError> {
    let x_field = match prefix {
        "mastering_red" => "mastering_red_x",
        "mastering_green" => "mastering_green_x",
        "mastering_blue" => "mastering_blue_x",
        "mastering_white_point" => "mastering_white_point_x",
        _ => prefix,
    };
    let y_field = match prefix {
        "mastering_red" => "mastering_red_y",
        "mastering_green" => "mastering_green_y",
        "mastering_blue" => "mastering_blue_y",
        "mastering_white_point" => "mastering_white_point_y",
        _ => prefix,
    };
    let x = number(x_field, x)?;
    let y = number(y_field, y)?;
    ChromaticityPoint::new_physical(x, y).ok_or_else(|| invalid_color_volume(prefix, "nonphysical"))
}

fn positive_number(
    field: &'static str,
    value: &str,
) -> Result<f64, Hdr10ColorVolumeValidationError> {
    let parsed = number(field, value)?;
    if parsed > 0.0 {
        Ok(parsed)
    } else {
        Err(invalid_color_volume(field, value))
    }
}

fn number(field: &'static str, value: &str) -> Result<f64, Hdr10ColorVolumeValidationError> {
    parse_number(value).ok_or_else(|| invalid_color_volume(field, value))
}

fn invalid_color_volume(field: &'static str, value: &str) -> Hdr10ColorVolumeValidationError {
    Hdr10ColorVolumeValidationError {
        field,
        value: value.to_string(),
    }
}

fn chromaticity_triangle_area(
    red: ChromaticityPoint,
    green: ChromaticityPoint,
    blue: ChromaticityPoint,
) -> f64 {
    (blue.x - red.x)
        .mul_add(-(green.y - red.y), (green.x - red.x) * (blue.y - red.y))
        .abs()
        / 2.0
}
