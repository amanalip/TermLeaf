//! Terminal-image backend selection, bounded transports, and cell rendering.

use std::{collections::BTreeMap, fmt::Write as _, io, io::Write, mem::size_of, sync::Arc};

use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Inclusive combined budget for a half-block frame and its ANSI output.
pub const HALF_BLOCK_ALLOCATION_LIMIT_BYTES: u64 = 256 * 1024 * 1024;

// Two longest truecolor SGR sequences, one UTF-8 block, a reset, and a newline.
const MAX_ANSI_BYTES_PER_CELL: u64 = 46;
const BYTES_RESERVED_PER_CELL: u64 = size_of::<HalfBlockCell>() as u64 + MAX_ANSI_BYTES_PER_CELL;

/// Maximum half-block cells whose frame and worst-case ANSI output fit the
/// image allocation budget.
pub const MAX_HALF_BLOCK_CELLS: u64 = HALF_BLOCK_ALLOCATION_LIMIT_BYTES / BYTES_RESERVED_PER_CELL;

/// Maximum retained wire bytes for one fitted native image.
pub const NATIVE_OUTPUT_LIMIT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum independently writable pieces retained for one native image.
pub const NATIVE_CHUNK_LIMIT: usize = 4096;
const KITTY_PAYLOAD_BYTES: usize = 4096;
const STREAM_CHUNK_BYTES: usize = 64 * 1024;
const KITTY_RAW_CHUNK_BYTES: usize = KITTY_PAYLOAD_BYTES / 4 * 3;
const ITERM_RAW_PART_BYTES: usize = STREAM_CHUNK_BYTES / 4 * 3;
const ARC_ALLOCATION_OVERHEAD: usize = size_of::<usize>() * 2;

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

impl ImageBackend {
    #[must_use]
    pub const fn is_native(self) -> bool {
        matches!(self, Self::Kitty | Self::Sixel | Self::Iterm2)
    }
}

/// Stable session-local identity for one logical image.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ImageId(u32);

impl ImageId {
    /// Creates a nonzero protocol-safe identifier.
    #[must_use]
    pub const fn new(value: u32) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Measured pixel dimensions of one terminal cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellPixelSize {
    pub width: u16,
    pub height: u16,
}

impl CellPixelSize {
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Option<Self> {
        if width == 0 || height == 0 {
            None
        } else {
            Some(Self { width, height })
        }
    }
}

/// Bounded, pre-encoded image output produced away from the UI thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeImage {
    id: ImageId,
    backend: ImageBackend,
    columns: u16,
    rows: u16,
    chunks: Vec<Vec<u8>>,
}

impl NativeImage {
    #[must_use]
    pub const fn id(&self) -> ImageId {
        self.id
    }

    #[must_use]
    pub const fn backend(&self) -> ImageBackend {
        self.backend
    }

    #[must_use]
    pub const fn columns(&self) -> u16 {
        self.columns
    }

    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }

    #[must_use]
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    #[must_use]
    pub fn wire_bytes(&self) -> usize {
        self.chunks.iter().map(Vec::len).sum()
    }

    #[must_use]
    pub fn allocation_bytes(&self) -> usize {
        self.chunks.iter().map(Vec::capacity).fold(
            ARC_ALLOCATION_OVERHEAD
                .saturating_add(size_of::<Self>())
                .saturating_add(self.chunks.capacity().saturating_mul(size_of::<Vec<u8>>())),
            usize::saturating_add,
        )
    }
}

/// One native image at absolute zero-based terminal cell coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePlacement {
    pub column: u16,
    pub row: u16,
    pub image: Arc<NativeImage>,
}

/// Native side-channel collected while Ratatui paints ordinary cells.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeFramePlan {
    placements: Vec<NativePlacement>,
}

impl NativeFramePlan {
    pub fn push(&mut self, placement: NativePlacement) {
        self.placements.push(placement);
    }

    #[must_use]
    pub fn placements(&self) -> &[NativePlacement] {
        &self.placements
    }
}

/// Native fitting or serialization failed before any terminal bytes were sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NativeEncodeError {
    #[error("native image dimensions and terminal bounds must be non-zero")]
    EmptyGeometry,
    #[error("native image source buffer is incomplete")]
    InvalidBuffer,
    #[error("native image output exceeds the {limit} byte limit")]
    OutputTooLarge { limit: usize },
    #[error("native image output exceeds the {limit} chunk limit")]
    TooManyChunks { limit: usize },
    #[error("native PNG encoding failed")]
    Png,
    #[error("native image encoding was cancelled")]
    Cancelled,
    #[error("Sixel output requires measured terminal cell pixel geometry")]
    MissingCellPixelSize,
    #[error("the selected image backend is not native")]
    NotNative,
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

/// Fits and encodes one decoded image for an already selected native backend.
/// Capability discovery is deliberately external to this pure operation.
///
/// # Errors
///
/// Returns a typed geometry, buffer, output-budget, chunk-budget, or PNG error.
pub fn encode_native_image(
    backend: ImageBackend,
    id: ImageId,
    image: &crate::document::DecodedImage,
    max_columns: u16,
    max_rows: u16,
    background: Rgb,
    cell_pixels: Option<CellPixelSize>,
) -> Result<NativeImage, NativeEncodeError> {
    encode_native_image_cancellable(
        backend,
        id,
        image,
        max_columns,
        max_rows,
        background,
        cell_pixels,
        || false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_native_image_cancellable(
    backend: ImageBackend,
    id: ImageId,
    image: &crate::document::DecodedImage,
    max_columns: u16,
    max_rows: u16,
    background: Rgb,
    cell_pixels: Option<CellPixelSize>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<NativeImage, NativeEncodeError> {
    if !backend.is_native() {
        return Err(NativeEncodeError::NotNative);
    }
    checkpoint(&mut cancelled)?;
    let (pixel_width, pixel_height, cell_width, cell_height) = match backend {
        ImageBackend::Sixel => {
            let cell = cell_pixels.ok_or(NativeEncodeError::MissingCellPixelSize)?;
            (
                u32::from(max_columns) * u32::from(cell.width),
                u32::from(max_rows) * u32::from(cell.height),
                u32::from(cell.width),
                u32::from(cell.height),
            )
        }
        ImageBackend::Kitty | ImageBackend::Iterm2 => {
            (u32::from(max_columns), u32::from(max_rows) * 2, 1, 2)
        }
        _ => return Err(NativeEncodeError::NotNative),
    };
    let (width, height, rgba) =
        fitted_rgba(image, pixel_width, pixel_height, background, &mut cancelled)?;
    let columns = u16::try_from(width.div_ceil(cell_width)).map_err(|_| {
        NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        }
    })?;
    let rows = u16::try_from(height.div_ceil(cell_height)).map_err(|_| {
        NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        }
    })?;
    let chunks = match backend {
        ImageBackend::Kitty => {
            kitty_chunks(id, width, height, columns, rows, &rgba, &mut cancelled)?
        }
        ImageBackend::Sixel => sixel_chunks(width, height, &rgba, &mut cancelled)?,
        ImageBackend::Iterm2 => {
            iterm2_chunks(id, columns, rows, width, height, &rgba, &mut cancelled)?
        }
        _ => return Err(NativeEncodeError::NotNative),
    };
    Ok(NativeImage {
        id,
        backend,
        columns,
        rows,
        chunks,
    })
}

fn fitted_rgba(
    image: &crate::document::DecodedImage,
    max_width: u32,
    max_height: u32,
    background: Rgb,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(u32, u32, Vec<u8>), NativeEncodeError> {
    if image.width() == 0 || image.height() == 0 || max_width == 0 || max_height == 0 {
        return Err(NativeEncodeError::EmptyGeometry);
    }
    let expected = u64::from(image.width())
        .checked_mul(u64::from(image.height()))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(NativeEncodeError::InvalidBuffer)?;
    if image.rgba().len() != expected {
        return Err(NativeEncodeError::InvalidBuffer);
    }
    let (width, height) =
        fit_native_dimensions(image.width(), image.height(), max_width, max_height);
    let output_len = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        })?;
    if output_len > NATIVE_OUTPUT_LIMIT_BYTES {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    let mut output = Vec::with_capacity(output_len);
    for y in 0..height {
        checkpoint(cancelled)?;
        for x in 0..width {
            let pixel = sample(
                image.rgba(),
                image.width(),
                image.height(),
                x,
                y,
                width,
                height,
                background,
            );
            output.extend_from_slice(&[pixel.0, pixel.1, pixel.2, 255]);
        }
    }
    Ok((width, height, output))
}

fn fit_native_dimensions(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    if u64::from(width) * u64::from(max_height) > u64::from(height) * u64::from(max_width) {
        let fitted_height = u32::try_from(
            ((u64::from(height) * u64::from(max_width) + u64::from(width) / 2) / u64::from(width))
                .clamp(1, u64::from(max_height)),
        )
        .unwrap_or(max_height);
        (max_width, fitted_height)
    } else {
        let fitted_width = u32::try_from(
            ((u64::from(width) * u64::from(max_height) + u64::from(height) / 2)
                / u64::from(height))
            .clamp(1, u64::from(max_width)),
        )
        .unwrap_or(max_width);
        (fitted_width, max_height)
    }
}

fn checkpoint(cancelled: &mut impl FnMut() -> bool) -> Result<(), NativeEncodeError> {
    if cancelled() {
        Err(NativeEncodeError::Cancelled)
    } else {
        Ok(())
    }
}

fn encoded_len(input: usize) -> Result<usize, NativeEncodeError> {
    input
        .checked_add(2)
        .map(|value| value / 3)
        .and_then(|groups| groups.checked_mul(4))
        .ok_or(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        })
}

fn kitty_chunks(
    id: ImageId,
    width: u32,
    height: u32,
    columns: u16,
    rows: u16,
    rgba: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Vec<Vec<u8>>, NativeEncodeError> {
    let base64_len = encoded_len(rgba.len())?;
    if base64_len > NATIVE_OUTPUT_LIMIT_BYTES {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    let count = rgba.len().div_ceil(KITTY_RAW_CHUNK_BYTES);
    if count > NATIVE_CHUNK_LIMIT {
        return Err(NativeEncodeError::TooManyChunks {
            limit: NATIVE_CHUNK_LIMIT,
        });
    }
    let mut chunks = Vec::with_capacity(count);
    for (index, raw) in rgba.chunks(KITTY_RAW_CHUNK_BYTES).enumerate() {
        checkpoint(cancelled)?;
        let payload = STANDARD.encode(raw);
        let more = usize::from(index + 1 < count);
        let prefix = if index == 0 {
            format!(
                "\x1b_Ga=T,t=d,f=32,s={width},v={height},i={},p={},c={columns},r={rows},q=2,m={more};",
                id.get(),
                id.get()
            )
        } else {
            format!("\x1b_Gm={more};")
        };
        let mut chunk = Vec::with_capacity(prefix.len() + payload.len() + 2);
        chunk.extend_from_slice(prefix.as_bytes());
        chunk.extend_from_slice(payload.as_bytes());
        chunk.extend_from_slice(b"\x1b\\");
        chunks.push(chunk);
    }
    check_native_chunks(&chunks)?;
    Ok(chunks)
}

struct BoundedPngWriter<'a, F> {
    bytes: Vec<u8>,
    cancelled: &'a mut F,
    was_cancelled: bool,
    exceeded: bool,
}

impl<F: FnMut() -> bool> Write for BoundedPngWriter<'_, F> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if (self.cancelled)() {
            self.was_cancelled = true;
            return Err(io::Error::other("cancelled"));
        }
        if self.bytes.len().saturating_add(bytes.len()) > NATIVE_OUTPUT_LIMIT_BYTES {
            self.exceeded = true;
            return Err(io::Error::new(io::ErrorKind::FileTooLarge, "PNG limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn png_bytes(
    width: u32,
    height: u32,
    rgba: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Vec<u8>, NativeEncodeError> {
    use image::ImageEncoder as _;

    let mut writer = BoundedPngWriter {
        bytes: Vec::new(),
        cancelled,
        was_cancelled: false,
        exceeded: false,
    };
    let result = image::codecs::png::PngEncoder::new(&mut writer).write_image(
        rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    );
    if writer.was_cancelled {
        return Err(NativeEncodeError::Cancelled);
    }
    if writer.exceeded {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    result.map_err(|_| NativeEncodeError::Png)?;
    Ok(writer.bytes)
}

fn iterm2_chunks(
    id: ImageId,
    columns: u16,
    rows: u16,
    width: u32,
    height: u32,
    rgba: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Vec<Vec<u8>>, NativeEncodeError> {
    let png = png_bytes(width, height, rgba, cancelled)?;
    if encoded_len(png.len())? > NATIVE_OUTPUT_LIMIT_BYTES {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    let name = STANDARD.encode(format!("termleaf-{}.png", id.get()));
    let mut chunks = vec![format!(
        "\x1b]1337;MultipartFile=inline=1;size={};width={columns};height={rows};preserveAspectRatio=1;name={name}\x07",
        png.len()
    )
    .into_bytes()];
    for raw in png.chunks(ITERM_RAW_PART_BYTES) {
        checkpoint(cancelled)?;
        let part = STANDARD.encode(raw);
        let mut chunk = Vec::with_capacity(part.len() + 23);
        chunk.extend_from_slice(b"\x1b]1337;FilePart=");
        chunk.extend_from_slice(part.as_bytes());
        chunk.push(0x07);
        chunks.push(chunk);
    }
    chunks.push(b"\x1b]1337;FileEnd\x07".to_vec());
    check_native_chunks(&chunks)?;
    Ok(chunks)
}

fn sixel_chunks(
    width: u32,
    height: u32,
    rgba: &[u8],
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Vec<Vec<u8>>, NativeEncodeError> {
    let mut indices = Vec::with_capacity(rgba.len() / 4);
    for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
        if index % 1024 == 0 {
            checkpoint(cancelled)?;
        }
        indices.push(nearest_xterm_index(Rgb(pixel[0], pixel[1], pixel[2])));
    }
    let mut palette = BTreeMap::new();
    for index in &indices {
        let next = u8::try_from(palette.len()).unwrap_or(u8::MAX);
        palette.entry(*index).or_insert(next);
    }
    let mut output = Vec::new();
    append_native(&mut output, b"\x1bP0;0;0q")?;
    append_native(&mut output, format!("\"1;1;{width};{height}").as_bytes())?;
    for (xterm, local) in &palette {
        let Rgb(red, green, blue) = xterm_rgb(*xterm);
        append_native(
            &mut output,
            format!(
                "#{local};2;{};{};{}",
                percent(red),
                percent(green),
                percent(blue)
            )
            .as_bytes(),
        )?;
    }
    let width_usize = usize::try_from(width).map_err(|_| NativeEncodeError::OutputTooLarge {
        limit: NATIVE_OUTPUT_LIMIT_BYTES,
    })?;
    for band in (0..height).step_by(6) {
        checkpoint(cancelled)?;
        for (palette_index, local) in &palette {
            checkpoint(cancelled)?;
            append_native(&mut output, format!("#{local}").as_bytes())?;
            let mut run_byte = 0_u8;
            let mut run_len = 0_usize;
            for column in 0..width {
                if column % 1024 == 0 {
                    checkpoint(cancelled)?;
                }
                let mut mask = 0_u8;
                for bit in 0..6_u32 {
                    let row = band + bit;
                    if row < height {
                        let offset = usize::try_from(row)
                            .unwrap_or(usize::MAX)
                            .saturating_mul(width_usize)
                            .saturating_add(usize::try_from(column).unwrap_or(usize::MAX));
                        if indices.get(offset) == Some(palette_index) {
                            mask |= 1 << bit;
                        }
                    }
                }
                let byte = b'?' + mask;
                if run_len == 0 || byte == run_byte {
                    run_byte = byte;
                    run_len += 1;
                } else {
                    append_sixel_run(&mut output, run_byte, run_len)?;
                    run_byte = byte;
                    run_len = 1;
                }
            }
            append_sixel_run(&mut output, run_byte, run_len)?;
            append_native(&mut output, b"$")?;
        }
        if band + 6 < height {
            append_native(&mut output, b"-")?;
        }
    }
    append_native(&mut output, b"\x1b\\")?;
    let chunks = output
        .chunks(STREAM_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    check_native_chunks(&chunks)?;
    Ok(chunks)
}

fn append_sixel_run(output: &mut Vec<u8>, byte: u8, count: usize) -> Result<(), NativeEncodeError> {
    if count >= 4 {
        append_native(output, format!("!{count}").as_bytes())?;
        append_native(output, &[byte])
    } else {
        for _ in 0..count {
            append_native(output, &[byte])?;
        }
        Ok(())
    }
}

fn append_native(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), NativeEncodeError> {
    if output.len().saturating_add(bytes.len()) > NATIVE_OUTPUT_LIMIT_BYTES {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn check_native_chunks(chunks: &[Vec<u8>]) -> Result<(), NativeEncodeError> {
    if chunks.len() > NATIVE_CHUNK_LIMIT {
        return Err(NativeEncodeError::TooManyChunks {
            limit: NATIVE_CHUNK_LIMIT,
        });
    }
    if chunks
        .iter()
        .map(Vec::len)
        .fold(0_usize, usize::saturating_add)
        > NATIVE_OUTPUT_LIMIT_BYTES
    {
        return Err(NativeEncodeError::OutputTooLarge {
            limit: NATIVE_OUTPUT_LIMIT_BYTES,
        });
    }
    Ok(())
}

const fn percent(value: u8) -> u16 {
    (value as u16 * 100 + 127) / 255
}

fn xterm_rgb(index: u8) -> Rgb {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        16..=231 => {
            let value = index - 16;
            Rgb(
                LEVELS[usize::from(value / 36)],
                LEVELS[usize::from(value % 36 / 6)],
                LEVELS[usize::from(value % 6)],
            )
        }
        232..=255 => {
            let level = 8 + (index - 232) * 10;
            Rgb(level, level, level)
        }
        _ => Rgb(0, 0, 0),
    }
}

/// Tracks native objects already emitted in the alternate screen.
#[derive(Debug, Default)]
pub struct NativeGraphicsSession {
    current: NativeFramePlan,
}

impl NativeGraphicsSession {
    /// Legacy protocols need a clear and complete Ratatui redraw when an
    /// existing frame changes because they have no object deletion command.
    #[must_use]
    pub fn requires_full_redraw(&self, next: &NativeFramePlan) -> bool {
        self.current != *next
            && !self.current.placements.is_empty()
            && self
                .current
                .placements
                .iter()
                .chain(next.placements.iter())
                .any(|placement| placement.image.backend() != ImageBackend::Kitty)
    }

    /// Emits only the native changes required after Ratatui flushed its frame.
    ///
    /// # Errors
    ///
    /// Returns the first write or flush error from the terminal sink.
    pub fn synchronize<W: Write>(
        &mut self,
        output: &mut W,
        next: NativeFramePlan,
    ) -> io::Result<()> {
        if self.current == next {
            return Ok(());
        }
        let previous = self.current.clone();
        let mut cleanup = previous.clone();
        for placement in &next.placements {
            if !cleanup.placements.contains(placement) {
                cleanup.placements.push(placement.clone());
            }
        }
        self.current = cleanup;
        let next_by_id = next
            .placements
            .iter()
            .map(|placement| (placement.image.id(), placement))
            .collect::<BTreeMap<_, _>>();
        for placement in &previous.placements {
            let replacement = next_by_id.get(&placement.image.id()).copied();
            if placement.image.backend() == ImageBackend::Kitty && replacement != Some(placement) {
                write_kitty_delete(output, placement.image.id())?;
            }
        }
        for placement in &next.placements {
            if !previous.placements.contains(placement) {
                write_placement(output, placement)?;
            }
        }
        output.flush()?;
        self.current = next;
        Ok(())
    }

    /// Removes every TermLeaf-owned image before leaving the alternate screen.
    ///
    /// # Errors
    ///
    /// Returns the first write or flush error from the terminal sink.
    pub fn cleanup<W: Write>(&mut self, output: &mut W) -> io::Result<()> {
        let mut legacy = false;
        for placement in &self.current.placements {
            if placement.image.backend() == ImageBackend::Kitty {
                write_kitty_delete(output, placement.image.id())?;
            } else {
                legacy = true;
            }
        }
        if legacy {
            output.write_all(b"\x1b[2J\x1b[H")?;
        }
        output.flush()?;
        self.current = NativeFramePlan::default();
        Ok(())
    }
}

fn write_placement<W: Write>(output: &mut W, placement: &NativePlacement) -> io::Result<()> {
    write!(
        output,
        "\x1b7\x1b[{};{}H",
        placement.row.saturating_add(1),
        placement.column.saturating_add(1)
    )?;
    for chunk in placement.image.chunks() {
        output.write_all(chunk)?;
    }
    output.write_all(b"\x1b8")
}

fn write_kitty_delete<W: Write>(output: &mut W, id: ImageId) -> io::Result<()> {
    write!(output, "\x1b_Ga=d,d=I,i={},q=2\x1b\\", id.get())
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

    #[must_use]
    pub(crate) fn allocation_bytes(&self) -> usize {
        ARC_ALLOCATION_OVERHEAD
            .saturating_add(size_of::<Self>())
            .saturating_add(
                self.cells
                    .capacity()
                    .saturating_mul(size_of::<HalfBlockCell>()),
            )
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

    fn decoded(width: u32, height: u32, pixels: Vec<u8>) -> crate::document::DecodedImage {
        let source = image::RgbaImage::from_raw(width, height, pixels).expect("pixel dimensions");
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .expect("encode PNG");
        crate::document::image::decode_bounded(&cursor.into_inner()).expect("decode PNG")
    }

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

    #[test]
    fn native_kitty_chunks_are_bounded_identified_and_protocol_exclusive() {
        let id = ImageId::new(7).expect("nonzero");
        let rgba = vec![255; 4_000];
        let chunks = kitty_chunks(id, 1_000, 1, 1_000, 1, &rgba, &mut || false).expect("bounded");

        assert_eq!(chunks.len(), 2, "payload crosses one 4096-byte boundary");
        assert!(chunks[0].starts_with(b"\x1b_Ga=T,t=d,f=32,s=1000,v=1,i=7,p=7"));
        assert!(chunks[0].windows(4).any(|part| part == b"m=1;"));
        assert!(chunks[1].starts_with(b"\x1b_Gm=0;"));
        assert!(chunks.iter().all(|chunk| chunk.ends_with(b"\x1b\\")));
        let wire = chunks.concat();
        assert!(!wire.windows(7).any(|part| part == b"1337;Fi"));
        assert!(!wire.starts_with(b"\x1bP"));
    }

    #[test]
    fn native_sixel_has_one_balanced_dcs_and_correct_top_pixel_bit() {
        let chunks = sixel_chunks(1, 1, &[255, 0, 0, 255], &mut || false).expect("sixel");
        let wire = chunks.concat();

        assert!(wire.starts_with(b"\x1bP0;0;0q\"1;1;1;1"));
        assert!(wire.ends_with(b"\x1b\\"));
        assert!(wire.windows(2).any(|part| part == b"#0"));
        assert!(wire.contains(&b'@'), "mask bit zero paints the top pixel");
        assert!(!wire.windows(3).any(|part| part == b"\x1b_G"));
        assert!(!wire.windows(6).any(|part| part == b"1337;F"));
    }

    #[test]
    fn native_sixel_requires_and_uses_measured_cell_pixel_geometry() {
        let image = decoded(16, 32, vec![255; 16 * 32 * 4]);
        let id = ImageId::new(1).expect("id");
        assert_eq!(
            encode_native_image(ImageBackend::Sixel, id, &image, 2, 1, Rgb(0, 0, 0), None,),
            Err(NativeEncodeError::MissingCellPixelSize)
        );

        let encoded = encode_native_image(
            ImageBackend::Sixel,
            id,
            &image,
            2,
            1,
            Rgb(0, 0, 0),
            CellPixelSize::new(8, 16),
        )
        .expect("measured Sixel");
        assert_eq!((encoded.columns(), encoded.rows()), (1, 1));
        assert!(
            encoded
                .chunks()
                .concat()
                .windows(b"\"1;1;8;16".len())
                .any(|window| window == b"\"1;1;8;16"),
            "Sixel raster dimensions are pixels, not terminal cells"
        );
    }

    #[test]
    fn native_encoding_cancels_during_fitting_png_and_sixel_work() {
        let image = decoded(64, 64, vec![128; 64 * 64 * 4]);
        let mut checks = 0;
        let result = encode_native_image_cancellable(
            ImageBackend::Kitty,
            ImageId::new(1).expect("id"),
            &image,
            64,
            32,
            Rgb(0, 0, 0),
            None,
            || {
                checks += 1;
                checks > 3
            },
        );
        assert_eq!(result, Err(NativeEncodeError::Cancelled));

        assert_eq!(
            png_bytes(1, 1, &[255; 4], &mut || true),
            Err(NativeEncodeError::Cancelled)
        );
        assert_eq!(
            sixel_chunks(1, 1, &[255; 4], &mut || true),
            Err(NativeEncodeError::Cancelled)
        );
    }

    #[test]
    fn native_png_writer_rejects_before_allocating_one_byte_over_limit() {
        let mut never = || false;
        let mut writer = BoundedPngWriter {
            bytes: vec![0; NATIVE_OUTPUT_LIMIT_BYTES],
            cancelled: &mut never,
            was_cancelled: false,
            exceeded: false,
        };
        assert_eq!(writer.write(&[]).expect("inclusive boundary"), 0);
        let error = writer.write(&[0]).expect_err("one over rejects");
        assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
        assert_eq!(writer.bytes.len(), NATIVE_OUTPUT_LIMIT_BYTES);
        assert!(writer.exceeded);
    }

    #[test]
    fn native_sixel_multicolor_bands_are_balanced_and_advance() {
        let mut rgba = Vec::new();
        for row in 0..7 {
            for column in 0..2 {
                rgba.extend_from_slice(if (row + column) % 2 == 0 {
                    &[255, 0, 0, 255]
                } else {
                    &[0, 0, 255, 255]
                });
            }
        }
        let wire = sixel_chunks(2, 7, &rgba, &mut || false)
            .expect("sixel")
            .concat();
        assert_eq!(wire.windows(2).filter(|part| *part == b"\x1bP").count(), 1);
        assert_eq!(wire.windows(2).filter(|part| *part == b"\x1b\\").count(), 1);
        assert!(wire.contains(&b'-'), "second six-pixel band advances");
        assert!(wire.windows(2).any(|part| part == b"#0"));
        assert!(wire.windows(2).any(|part| part == b"#1"));
    }

    #[test]
    fn native_iterm2_uses_bounded_multipart_framing_and_generated_name() {
        let chunks = iterm2_chunks(
            ImageId::new(9).expect("id"),
            2,
            1,
            2,
            2,
            &[255; 16],
            &mut || false,
        )
        .expect("multipart");

        assert!(chunks[0].starts_with(b"\x1b]1337;MultipartFile=inline=1;"));
        assert!(chunks[0].windows(5).any(|part| part == b"name="));
        assert!(chunks[1].starts_with(b"\x1b]1337;FilePart="));
        assert_eq!(chunks.last().expect("end"), b"\x1b]1337;FileEnd\x07");
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.len() <= STREAM_CHUNK_BYTES + 64)
        );
        let wire = chunks.concat();
        assert!(!wire.windows(3).any(|part| part == b"\x1b_G"));
        assert!(!wire.starts_with(b"\x1bP"));
    }

    fn native_for_test(id: u32, backend: ImageBackend, marker: u8) -> Arc<NativeImage> {
        Arc::new(NativeImage {
            id: ImageId::new(id).expect("id"),
            backend,
            columns: 1,
            rows: 1,
            chunks: vec![vec![marker]],
        })
    }

    #[test]
    fn native_lifecycle_replaces_deletes_and_suppresses_unchanged_frames() {
        let first = NativePlacement {
            column: 3,
            row: 4,
            image: native_for_test(5, ImageBackend::Kitty, b'A'),
        };
        let mut session = NativeGraphicsSession::default();
        let mut output = Vec::new();
        session
            .synchronize(
                &mut output,
                NativeFramePlan {
                    placements: vec![first.clone()],
                },
            )
            .expect("first frame");
        assert!(output.windows(5).any(|part| part == b"\x1b[5;4"));
        assert!(output.contains(&b'A'));

        output.clear();
        session
            .synchronize(
                &mut output,
                NativeFramePlan {
                    placements: vec![first],
                },
            )
            .expect("unchanged");
        assert!(output.is_empty(), "10 Hz redraws do not retransmit images");

        let replacement = NativePlacement {
            column: 6,
            row: 7,
            image: native_for_test(5, ImageBackend::Kitty, b'B'),
        };
        session
            .synchronize(
                &mut output,
                NativeFramePlan {
                    placements: vec![replacement],
                },
            )
            .expect("replacement");
        let delete = output
            .windows(4)
            .position(|part| part == b"a=d,")
            .expect("delete command");
        let replacement = output
            .iter()
            .position(|byte| *byte == b'B')
            .expect("new image");
        assert!(delete < replacement, "delete precedes ID reuse");

        output.clear();
        session.cleanup(&mut output).expect("cleanup");
        assert!(output.windows(4).any(|part| part == b"a=d,"));
    }

    #[derive(Default)]
    struct FailingWriter {
        writes: usize,
        fail_write_at: Option<usize>,
        fail_flush: bool,
        bytes: Vec<u8>,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let call = self.writes;
            self.writes += 1;
            if self.fail_write_at == Some(call) {
                return Err(io::Error::other("injected write failure"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::Error::other("injected flush failure"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn native_lifecycle_failures_keep_every_attempted_id_cleanup_safe() {
        let plan = NativeFramePlan {
            placements: vec![NativePlacement {
                column: 0,
                row: 0,
                image: native_for_test(3, ImageBackend::Kitty, b'K'),
            }],
        };
        for mut writer in [
            FailingWriter {
                fail_write_at: Some(0),
                ..FailingWriter::default()
            },
            FailingWriter {
                fail_flush: true,
                ..FailingWriter::default()
            },
        ] {
            let mut session = NativeGraphicsSession::default();
            assert!(session.synchronize(&mut writer, plan.clone()).is_err());
            let mut cleanup = Vec::new();
            session.cleanup(&mut cleanup).expect("retry cleanup");
            assert!(
                cleanup.windows(4).any(|part| part == b"a=d,"),
                "partially written IDs remain tracked"
            );
        }

        let mut session = NativeGraphicsSession::default();
        session
            .synchronize(&mut Vec::new(), plan)
            .expect("initial placement");
        let replacement = NativeFramePlan {
            placements: vec![NativePlacement {
                column: 1,
                row: 1,
                image: native_for_test(3, ImageBackend::Kitty, b'R'),
            }],
        };
        let mut writer = FailingWriter {
            fail_write_at: Some(0),
            ..FailingWriter::default()
        };
        assert!(session.synchronize(&mut writer, replacement).is_err());
        let mut cleanup = Vec::new();
        session.cleanup(&mut cleanup).expect("replacement cleanup");
        assert!(cleanup.windows(4).any(|part| part == b"a=d,"));
    }

    #[test]
    fn legacy_lifecycle_requires_redraw_then_clears_on_shutdown() {
        let plan = NativeFramePlan {
            placements: vec![NativePlacement {
                column: 0,
                row: 0,
                image: native_for_test(1, ImageBackend::Sixel, b'S'),
            }],
        };
        let mut session = NativeGraphicsSession::default();
        assert!(
            !session.requires_full_redraw(&plan),
            "first frame has no stale pixels"
        );
        session.synchronize(&mut Vec::new(), plan).expect("first");
        assert!(session.requires_full_redraw(&NativeFramePlan::default()));

        let mut output = Vec::new();
        session.cleanup(&mut output).expect("clear");
        assert_eq!(output, b"\x1b[2J\x1b[H");
    }

    #[test]
    fn native_output_and_chunk_boundaries_reject_one_over() {
        let mut exact = vec![0; NATIVE_OUTPUT_LIMIT_BYTES];
        assert_eq!(append_native(&mut exact, &[]), Ok(()));
        assert_eq!(
            append_native(&mut exact, &[0]),
            Err(NativeEncodeError::OutputTooLarge {
                limit: NATIVE_OUTPUT_LIMIT_BYTES
            })
        );
        let too_many = vec![Vec::new(); NATIVE_CHUNK_LIMIT + 1];
        assert_eq!(
            check_native_chunks(&too_many),
            Err(NativeEncodeError::TooManyChunks {
                limit: NATIVE_CHUNK_LIMIT
            })
        );
    }
}
