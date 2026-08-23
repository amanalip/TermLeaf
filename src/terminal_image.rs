//! Pure terminal-image backend selection and cell fallback rendering.

use std::{fmt::Write, mem::size_of};

/// Inclusive combined budget for a half-block frame and its ANSI output.
pub const HALF_BLOCK_ALLOCATION_LIMIT_BYTES: u64 = 256 * 1024 * 1024;

// Two longest truecolor SGR sequences, one UTF-8 block, a reset, and a newline.
const MAX_ANSI_BYTES_PER_CELL: u64 = 46;
const BYTES_RESERVED_PER_CELL: u64 = size_of::<HalfBlockCell>() as u64 + MAX_ANSI_BYTES_PER_CELL;

/// Maximum half-block cells whose frame and worst-case ANSI output fit the
/// image allocation budget.
pub const MAX_HALF_BLOCK_CELLS: u64 = HALF_BLOCK_ALLOCATION_LIMIT_BYTES / BYTES_RESERVED_PER_CELL;

/// A terminal image output path, in automatic preference order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageBackend {
    Kitty,
    Sixel,
    Iterm2,
    TrueColorCells,
    Ansi256Cells,
    Caption,
}

/// Evidence reported for one terminal capability.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CapabilityEvidence {
    Positive,
    Negative,
    Malformed,
    #[default]
    Absent,
}

impl CapabilityEvidence {
    /// Parses a normalized probe result. Unknown non-empty reports are not
    /// treated as support: emitting a protocol requires positive evidence.
    #[must_use]
    pub fn from_report(report: Option<&str>) -> Self {
        let Some(report) = report.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self::Absent;
        };
        match report.to_ascii_lowercase().as_str() {
            "1" | "yes" | "true" | "supported" | "ok" => Self::Positive,
            "0" | "no" | "false" | "unsupported" => Self::Negative,
            _ => Self::Malformed,
        }
    }
}

/// Capability reports collected before rendering begins.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImageCapabilities {
    pub kitty: CapabilityEvidence,
    pub sixel: CapabilityEvidence,
    pub iterm2: CapabilityEvidence,
    pub true_color: CapabilityEvidence,
    pub ansi256: CapabilityEvidence,
}

/// An explicit output path was contradicted by terminal capability evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("image backend override {backend:?} is incompatible with terminal evidence {evidence:?}")]
pub struct IncompatibleImageOverride {
    pub backend: ImageBackend,
    pub evidence: CapabilityEvidence,
}

/// Resolves an explicit override or the safest automatically supported path.
///
/// An override takes precedence over every automatic candidate. Absent or
/// malformed reports never enable an automatic protocol. An explicit override
/// can intentionally replace absent detection, but a negative report is a
/// typed incompatibility rather than a silent fallback (`DEC-TEST-014`).
///
/// # Errors
///
/// Returns [`IncompatibleImageOverride`] when an override has explicit
/// negative capability evidence.
pub fn select_backend(
    explicit: Option<ImageBackend>,
    capabilities: ImageCapabilities,
) -> Result<ImageBackend, IncompatibleImageOverride> {
    if let Some(backend) = explicit {
        let evidence = evidence_for(backend, capabilities);
        if evidence == CapabilityEvidence::Negative {
            return Err(IncompatibleImageOverride { backend, evidence });
        }
        return Ok(backend);
    }

    for (backend, evidence) in [
        (ImageBackend::Kitty, capabilities.kitty),
        (ImageBackend::Sixel, capabilities.sixel),
        (ImageBackend::Iterm2, capabilities.iterm2),
        (ImageBackend::TrueColorCells, capabilities.true_color),
        (ImageBackend::Ansi256Cells, capabilities.ansi256),
    ] {
        if evidence == CapabilityEvidence::Positive {
            return Ok(backend);
        }
    }
    Ok(ImageBackend::Caption)
}

const fn evidence_for(
    backend: ImageBackend,
    capabilities: ImageCapabilities,
) -> CapabilityEvidence {
    match backend {
        ImageBackend::Kitty => capabilities.kitty,
        ImageBackend::Sixel => capabilities.sixel,
        ImageBackend::Iterm2 => capabilities.iterm2,
        ImageBackend::TrueColorCells => capabilities.true_color,
        ImageBackend::Ansi256Cells => capabilities.ansi256,
        ImageBackend::Caption => CapabilityEvidence::Positive,
    }
}

/// RGB color used by cell rendering and alpha compositing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Color representation emitted for a half-block frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellColor {
    Rgb(Rgb),
    Indexed(u8),
}

/// Requested cell color depth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellColorMode {
    TrueColor,
    Ansi256,
}

/// One Unicode upper-half block cell. The foreground paints the upper source
/// pixel and the background paints the lower source pixel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HalfBlockCell {
    pub foreground: CellColor,
    pub background: CellColor,
}

/// A row-major, bounds-fitted half-block image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HalfBlockFrame {
    width: u16,
    height: u16,
    cells: Vec<HalfBlockCell>,
}

impl HalfBlockFrame {
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn cells(&self) -> &[HalfBlockCell] {
        &self.cells
    }

    /// Serializes the frame as SGR-colored upper-half block rows.
    #[must_use]
    pub fn ansi(&self) -> String {
        let capacity = self
            .cells
            .len()
            .saturating_mul(usize::try_from(MAX_ANSI_BYTES_PER_CELL).unwrap_or(usize::MAX));
        let mut output = String::with_capacity(capacity);
        for (row_index, row) in self.cells.chunks(usize::from(self.width)).enumerate() {
            for cell in row {
                write_color(&mut output, cell.foreground, true);
                write_color(&mut output, cell.background, false);
                output.push('\u{2580}');
            }
            output.push_str("\x1b[0m");
            if row_index + 1 < usize::from(self.height) {
                output.push('\n');
            }
        }
        output
    }
}

/// Invalid source pixels or unusable output bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CellRenderError {
    #[error("image dimensions and terminal bounds must be non-zero")]
    EmptyGeometry,
    #[error("RGBA buffer has {actual} bytes, expected {expected}")]
    InvalidBuffer { expected: usize, actual: usize },
    #[error(
        "half-block output requires {requested} bytes beyond the {limit} byte allocation limit"
    )]
    AllocationTooLarge { requested: u64, limit: u64 },
}

/// Renders an image produced by `TermLeaf`'s bounded decoder.
///
/// # Errors
///
/// Returns [`CellRenderError`] if the decoded image has unusable geometry.
pub fn render_decoded_half_blocks(
    image: &crate::document::DecodedImage,
    max_columns: u16,
    max_rows: u16,
    background: Rgb,
    mode: CellColorMode,
) -> Result<HalfBlockFrame, CellRenderError> {
    render_half_blocks(
        image.width(),
        image.height(),
        image.rgba(),
        max_columns,
        max_rows,
        background,
        mode,
    )
}

/// Fits RGBA8 source pixels into terminal bounds and renders `▀` cells.
///
/// Source pixels are never mutated. Nearest-neighbor fitting preserves aspect
/// ratio within `max_columns` by `max_rows * 2` pixel space. Alpha is composited
/// against `background` before optional xterm-256 quantization.
///
/// # Errors
///
/// Returns [`CellRenderError`] for zero geometry, output beyond the allocation
/// budget, or an incomplete RGBA buffer.
pub fn render_half_blocks(
    width: u32,
    height: u32,
    rgba: &[u8],
    max_columns: u16,
    max_rows: u16,
    background: Rgb,
    mode: CellColorMode,
) -> Result<HalfBlockFrame, CellRenderError> {
    if width == 0 || height == 0 || max_columns == 0 || max_rows == 0 {
        return Err(CellRenderError::EmptyGeometry);
    }

    let (fitted_width, fitted_height) = fit_dimensions(width, height, max_columns, max_rows);
    let cell_rows = fitted_height.div_ceil(2);
    let cell_count = u64::from(fitted_width) * u64::from(cell_rows);
    check_cell_limit(cell_count)?;

    let expected = usize::try_from(
        u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(4),
    )
    .unwrap_or(usize::MAX);
    if rgba.len() != expected {
        return Err(CellRenderError::InvalidBuffer {
            expected,
            actual: rgba.len(),
        });
    }

    let capacity = usize::try_from(fitted_width)
        .unwrap_or(usize::MAX)
        .saturating_mul(usize::try_from(cell_rows).unwrap_or(usize::MAX));
    let mut cells = Vec::with_capacity(capacity);
    for row in 0..cell_rows {
        for column in 0..fitted_width {
            let upper = sample(
                rgba,
                width,
                height,
                column,
                row * 2,
                fitted_width,
                fitted_height,
                background,
            );
            let lower = if row * 2 + 1 < fitted_height {
                sample(
                    rgba,
                    width,
                    height,
                    column,
                    row * 2 + 1,
                    fitted_width,
                    fitted_height,
                    background,
                )
            } else {
                background
            };
            cells.push(HalfBlockCell {
                foreground: convert_color(upper, mode),
                background: convert_color(lower, mode),
            });
        }
    }

    Ok(HalfBlockFrame {
        width: u16::try_from(fitted_width).unwrap_or(max_columns),
        height: u16::try_from(cell_rows).unwrap_or(max_rows),
        cells,
    })
}

fn check_cell_limit(cell_count: u64) -> Result<(), CellRenderError> {
    if cell_count > MAX_HALF_BLOCK_CELLS {
        return Err(CellRenderError::AllocationTooLarge {
            requested: cell_count.saturating_mul(BYTES_RESERVED_PER_CELL),
            limit: HALF_BLOCK_ALLOCATION_LIMIT_BYTES,
        });
    }
    Ok(())
}

fn fit_dimensions(width: u32, height: u32, max_columns: u16, max_rows: u16) -> (u32, u32) {
    let bound_width = u32::from(max_columns);
    let bound_height = u32::from(max_rows) * 2;
    if u64::from(width) * u64::from(bound_height) > u64::from(height) * u64::from(bound_width) {
        let fitted_height = u32::try_from(
            ((u64::from(height) * u64::from(bound_width) + u64::from(width) / 2)
                / u64::from(width))
            .clamp(1, u64::from(bound_height)),
        )
        .unwrap_or(bound_height);
        (bound_width, fitted_height)
    } else {
        let fitted_width = u32::try_from(
            ((u64::from(width) * u64::from(bound_height) + u64::from(height) / 2)
                / u64::from(height))
            .clamp(1, u64::from(bound_width)),
        )
        .unwrap_or(bound_width);
        (fitted_width, bound_height)
    }
}

#[allow(clippy::too_many_arguments)]
fn sample(
    rgba: &[u8],
    source_width: u32,
    source_height: u32,
    x: u32,
    y: u32,
    fitted_width: u32,
    fitted_height: u32,
    background: Rgb,
) -> Rgb {
    let source_x = u32::try_from(u64::from(x) * u64::from(source_width) / u64::from(fitted_width))
        .unwrap_or(source_width - 1);
    let source_y =
        u32::try_from(u64::from(y) * u64::from(source_height) / u64::from(fitted_height))
            .unwrap_or(source_height - 1);
    let offset =
        usize::try_from((u64::from(source_y) * u64::from(source_width) + u64::from(source_x)) * 4)
            .unwrap_or(usize::MAX);
    composite(
        Rgb(rgba[offset], rgba[offset + 1], rgba[offset + 2]),
        rgba[offset + 3],
        background,
    )
}

fn composite(source: Rgb, alpha: u8, background: Rgb) -> Rgb {
    let blend = |source: u8, background: u8| {
        let alpha = u32::from(alpha);
        u8::try_from(
            (u32::from(source) * alpha + u32::from(background) * (255 - alpha) + 127) / 255,
        )
        .unwrap_or(u8::MAX)
    };
    Rgb(
        blend(source.0, background.0),
        blend(source.1, background.1),
        blend(source.2, background.2),
    )
}

fn convert_color(color: Rgb, mode: CellColorMode) -> CellColor {
    match mode {
        CellColorMode::TrueColor => CellColor::Rgb(color),
        CellColorMode::Ansi256 => CellColor::Indexed(nearest_xterm_index(color)),
    }
}

/// Finds the nearest xterm 6x6x6 cube or grayscale-ramp entry.
#[must_use]
pub fn nearest_xterm_index(Rgb(red, green, blue): Rgb) -> u8 {
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let distance = |first: i32, second: i32| (first - second) * (first - second);
    let channel = |value: u8| {
        LEVELS
            .iter()
            .enumerate()
            .min_by_key(|&(index, level)| (distance(i32::from(value), *level), index))
            .map_or(0, |(index, _)| index)
    };
    let red_index = channel(red);
    let green_index = channel(green);
    let blue_index = channel(blue);
    let cube = 16 + 36 * red_index + 6 * green_index + blue_index;
    let cube_distance = distance(i32::from(red), LEVELS[red_index])
        + distance(i32::from(green), LEVELS[green_index])
        + distance(i32::from(blue), LEVELS[blue_index]);

    let (gray_step, gray_distance) = (0..24_i32)
        .map(|step| {
            let level = 8 + step * 10;
            (
                step,
                distance(i32::from(red), level)
                    + distance(i32::from(green), level)
                    + distance(i32::from(blue), level),
            )
        })
        .min_by_key(|&(step, distance)| (distance, step))
        .unwrap_or((0, i32::MAX));
    if cube_distance <= gray_distance {
        u8::try_from(cube).unwrap_or(u8::MAX)
    } else {
        u8::try_from(232 + gray_step).unwrap_or(u8::MAX)
    }
}

fn write_color(output: &mut String, color: CellColor, foreground: bool) {
    let prefix = if foreground { 38 } else { 48 };
    match color {
        CellColor::Rgb(Rgb(red, green, blue)) => {
            let _ = write!(output, "\x1b[{prefix};2;{red};{green};{blue}m");
        }
        CellColor::Indexed(index) => {
            let _ = write!(output, "\x1b[{prefix};5;{index}m");
        }
    }
}

/// Builds a useful one-line caption after image loading or rendering fails.
#[must_use]
pub fn failure_caption(alt: Option<&str>, dimensions: Option<(u32, u32)>, reason: &str) -> String {
    let alt = alt.map(str::trim).filter(|value| !value.is_empty());
    let normalized_reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated = normalized_reason.chars().count() > 80;
    let mut reason = normalized_reason.chars().take(80).collect::<String>();
    if truncated {
        reason.push_str("...");
    }
    if reason.is_empty() {
        reason.push_str("could not display");
    }

    let mut parts = Vec::new();
    if let Some(alt) = alt {
        parts.push(alt.to_owned());
    }
    if let Some((width, height)) = dimensions {
        parts.push(format!("{width}x{height}"));
    }
    parts.push(reason);
    format!("[image: {}]", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_override_wins_and_negative_evidence_is_typed() {
        let capabilities = ImageCapabilities {
            kitty: CapabilityEvidence::Positive,
            sixel: CapabilityEvidence::Negative,
            true_color: CapabilityEvidence::Positive,
            ..ImageCapabilities::default()
        };
        assert_eq!(
            select_backend(Some(ImageBackend::Caption), capabilities),
            Ok(ImageBackend::Caption)
        );
        assert_eq!(
            select_backend(Some(ImageBackend::Sixel), capabilities),
            Err(IncompatibleImageOverride {
                backend: ImageBackend::Sixel,
                evidence: CapabilityEvidence::Negative,
            })
        );
        assert_eq!(
            select_backend(Some(ImageBackend::Iterm2), capabilities),
            Ok(ImageBackend::Iterm2),
            "an override may replace absent detection"
        );
    }

    #[test]
    fn automatic_selection_requires_positive_evidence_in_locked_order() {
        let mut capabilities = ImageCapabilities {
            kitty: CapabilityEvidence::Malformed,
            sixel: CapabilityEvidence::Positive,
            iterm2: CapabilityEvidence::Positive,
            true_color: CapabilityEvidence::Positive,
            ..ImageCapabilities::default()
        };
        assert_eq!(select_backend(None, capabilities), Ok(ImageBackend::Sixel));
        capabilities.sixel = CapabilityEvidence::Absent;
        assert_eq!(select_backend(None, capabilities), Ok(ImageBackend::Iterm2));
        capabilities.iterm2 = CapabilityEvidence::Negative;
        assert_eq!(
            select_backend(None, capabilities),
            Ok(ImageBackend::TrueColorCells)
        );
        capabilities.true_color = CapabilityEvidence::Malformed;
        assert_eq!(
            select_backend(None, capabilities),
            Ok(ImageBackend::Caption),
            "absent ANSI-256 evidence does not enable cells"
        );
        capabilities.ansi256 = CapabilityEvidence::Positive;
        assert_eq!(
            select_backend(None, capabilities),
            Ok(ImageBackend::Ansi256Cells)
        );
        capabilities.ansi256 = CapabilityEvidence::Malformed;
        assert_eq!(
            select_backend(None, capabilities),
            Ok(ImageBackend::Caption)
        );
    }

    #[test]
    fn half_blocks_fit_bounds_composite_alpha_and_preserve_source_pixels() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 128, // top row
            0, 0, 255, 0, 255, 255, 255, 255, // bottom row
        ];
        let original = pixels.clone();
        let frame = render_half_blocks(
            2,
            2,
            &pixels,
            2,
            1,
            Rgb(10, 20, 30),
            CellColorMode::TrueColor,
        )
        .expect("valid pixels render");
        assert_eq!((frame.width(), frame.height()), (2, 1));
        assert_eq!(
            frame.cells(),
            &[
                HalfBlockCell {
                    foreground: CellColor::Rgb(Rgb(255, 0, 0)),
                    background: CellColor::Rgb(Rgb(10, 20, 30)),
                },
                HalfBlockCell {
                    foreground: CellColor::Rgb(Rgb(5, 138, 15)),
                    background: CellColor::Rgb(Rgb(255, 255, 255)),
                },
            ]
        );
        assert_eq!(pixels, original, "presentation never changes source pixels");
        assert!(frame.ansi().contains('\u{2580}'));
    }

    #[test]
    fn aspect_ratio_and_odd_pixel_rows_stay_inside_bounds() {
        let wide = vec![255; 8 * 2 * 4];
        let frame = render_half_blocks(8, 2, &wide, 4, 4, Rgb(0, 0, 0), CellColorMode::TrueColor)
            .expect("wide image fits");
        assert_eq!((frame.width(), frame.height()), (4, 1));

        let tall = vec![255; 2 * 9 * 4];
        let frame = render_half_blocks(2, 9, &tall, 20, 2, Rgb(0, 0, 0), CellColorMode::TrueColor)
            .expect("tall image fits");
        assert!(frame.width() <= 20 && frame.height() <= 2);
    }

    #[test]
    fn half_block_allocation_limit_accepts_boundary_and_rejects_one_over() {
        assert_eq!(
            HALF_BLOCK_ALLOCATION_LIMIT_BYTES,
            crate::document::ImageLimits::default().max_allocation_bytes,
            "cell output tracks the bounded decoder allocation policy"
        );
        assert_eq!(check_cell_limit(MAX_HALF_BLOCK_CELLS), Ok(()));
        assert_eq!(
            check_cell_limit(MAX_HALF_BLOCK_CELLS + 1),
            Err(CellRenderError::AllocationTooLarge {
                requested: (MAX_HALF_BLOCK_CELLS + 1) * BYTES_RESERVED_PER_CELL,
                limit: HALF_BLOCK_ALLOCATION_LIMIT_BYTES,
            })
        );

        let requested_cells = u64::from(u16::MAX) * u64::from(u16::MAX.div_ceil(2));
        assert_eq!(
            render_half_blocks(
                1,
                1,
                &[],
                u16::MAX,
                u16::MAX,
                Rgb(0, 0, 0),
                CellColorMode::TrueColor,
            ),
            Err(CellRenderError::AllocationTooLarge {
                requested: requested_cells * BYTES_RESERVED_PER_CELL,
                limit: HALF_BLOCK_ALLOCATION_LIMIT_BYTES,
            }),
            "huge bounds reject before buffer validation or allocation"
        );
    }

    #[test]
    fn ansi256_quantization_and_caption_content_are_deterministic() {
        assert_eq!(nearest_xterm_index(Rgb(255, 0, 0)), 196);
        assert_eq!(nearest_xterm_index(Rgb(128, 128, 128)), 244);
        let frame = render_half_blocks(
            1,
            1,
            &[255, 0, 0, 255],
            1,
            1,
            Rgb(0, 0, 0),
            CellColorMode::Ansi256,
        )
        .expect("one pixel renders");
        assert_eq!(frame.cells()[0].foreground, CellColor::Indexed(196));
        assert!(frame.ansi().contains("\x1b[38;5;196m"));

        assert_eq!(
            failure_caption(
                Some(" cover art "),
                Some((640, 480)),
                "decoder stopped early"
            ),
            "[image: cover art; 640x480; decoder stopped early]"
        );
        assert_eq!(
            failure_caption(None, None, ""),
            "[image: could not display]"
        );
    }

    #[test]
    fn reports_parse_without_promoting_malformed_values() {
        assert_eq!(
            CapabilityEvidence::from_report(Some(" SUPPORTED ")),
            CapabilityEvidence::Positive
        );
        assert_eq!(
            CapabilityEvidence::from_report(Some("maybe")),
            CapabilityEvidence::Malformed
        );
        assert_eq!(
            CapabilityEvidence::from_report(None),
            CapabilityEvidence::Absent
        );
    }
}
