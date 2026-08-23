//! Bounded, static SVG and SVGZ decoding.

use std::io::Read;

use flate2::bufread::GzDecoder;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{ImageHrefResolver, Options, Tree};

use super::{DecodedImage, ImageLimits, XmlLimits, XmlStructureError, validate_xml_structure};

/// Vector container selected from content evidence or an explicit declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorFormat {
    /// Uncompressed UTF-8 SVG XML.
    Svg,
    /// Gzip-compressed SVG XML.
    Svgz,
}

/// Inclusive parser and rasterizer work limits beyond the shared image limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorLimits {
    /// XML nesting and element-opening limits.
    pub xml: XmlLimits,
    /// Maximum total bytes in all path `d` attributes.
    pub max_path_data_bytes: usize,
    /// Maximum command letters in all path `d` attributes.
    pub max_path_commands: usize,
    /// Maximum transform functions in all `transform` attributes.
    pub max_transform_operations: usize,
    /// Maximum bytes in any one XML attribute value.
    pub max_attribute_bytes: usize,
}

impl Default for VectorLimits {
    fn default() -> Self {
        Self {
            xml: XmlLimits {
                max_depth: 128,
                max_nodes: 100_000,
            },
            max_path_data_bytes: 1024 * 1024,
            max_path_commands: 100_000,
            max_transform_operations: 10_000,
            max_attribute_bytes: 1024 * 1024,
        }
    }
}

/// Kind of syntactic work rejected before `resvg` parsing or rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorWork {
    /// Bytes across path-data attributes.
    PathDataBytes,
    /// Path command letters.
    PathCommands,
    /// Transform functions.
    TransformOperations,
    /// Bytes in one attribute value.
    AttributeBytes,
}

/// Typed SVG/SVGZ policy, parse, and render failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VectorImageError {
    /// Compressed or general input exceeded the gate shared by all images.
    #[error("vector resource is too large: {size} bytes exceeds the {limit} byte input limit")]
    InputTooLarge { size: u64, limit: u64 },
    /// Input had neither gzip magic nor an SVG root/declaration.
    #[error("unsupported vector data: expected SVG XML or an SVGZ gzip stream")]
    UnsupportedFormat,
    /// Actual XML bytes crossed the inclusive decompression limit.
    #[error("SVG XML is too large: more than {limit} decompressed bytes")]
    XmlTooLarge { limit: u64 },
    /// The gzip stream ended before its member and trailer were complete.
    #[error("SVGZ stream is truncated")]
    GzipTruncated,
    /// The gzip trailer did not match the decompressed stream.
    #[error("SVGZ checksum does not match its decompressed data")]
    GzipChecksum,
    /// The gzip header or deflate stream was malformed.
    #[error("SVGZ stream is corrupt")]
    GzipCorrupt,
    /// Bytes remained after the single permitted gzip member.
    #[error("SVGZ contains trailing or concatenated data")]
    GzipTrailingData,
    /// SVG XML was not UTF-8.
    #[error("SVG XML is not valid UTF-8")]
    InvalidUtf8,
    /// XML structure or declarations violated the pre-parse policy.
    #[error("unsafe SVG XML structure: {source}")]
    UnsafeXml { source: XmlStructureError },
    /// An executable or animated element was present.
    #[error("SVG element '{name}' is not allowed in a static image")]
    ForbiddenElement { name: String },
    /// An event or animation attribute was present.
    #[error("SVG attribute '{name}' is not allowed in a static image")]
    ForbiddenAttribute { name: String },
    /// A non-local resource reference was present.
    #[error("external SVG resource reference in '{attribute}' is not allowed")]
    ExternalReference { attribute: String },
    /// Filters and style sheets are excluded because their work/resource costs
    /// cannot be bounded tightly enough before rendering.
    #[error("unsupported bounded SVG feature: {feature}")]
    UnsupportedFeature { feature: &'static str },
    /// Syntactic work crossed an inclusive limit.
    #[error("SVG {work:?} work {observed} exceeds the {limit} limit")]
    WorkLimit {
        work: VectorWork,
        observed: usize,
        limit: usize,
    },
    /// Safe XML did not parse as a supported SVG tree.
    #[error("could not parse bounded SVG: {detail}")]
    ParseFailed { detail: String },
    /// A declared side crossed the shared image dimension limit.
    #[error("SVG declares {width}x{height} pixels beyond the {limit} pixel dimension limit")]
    DimensionTooLarge { width: u32, height: u32, limit: u32 },
    /// Output area crossed the shared pixel limit.
    #[error("SVG declares {pixels} pixels beyond the {limit} pixel limit")]
    TooManyPixels { pixels: u64, limit: u64 },
    /// The RGBA8 output crossed the shared allocation limit.
    #[error("SVG RGBA output requires {bytes} bytes beyond the {limit} byte allocation limit")]
    AllocationTooLarge { bytes: u64, limit: u64 },
    /// The already-budgeted pixmap could not be allocated.
    #[error("could not allocate the bounded SVG output pixmap")]
    RenderAllocationFailed,
}

/// Identifies SVG/SVGZ from leading content without parsing or decompressing it.
#[must_use]
pub fn sniff_vector_format(bytes: &[u8]) -> Option<VectorFormat> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        Some(VectorFormat::Svgz)
    } else if looks_like_svg(bytes) {
        Some(VectorFormat::Svg)
    } else {
        None
    }
}

/// Decodes SVG or SVGZ under the default image and vector policies.
///
/// # Errors
///
/// Returns a typed [`VectorImageError`] for every bounded policy or decode failure.
pub fn decode_vector_bounded(bytes: &[u8]) -> Result<DecodedImage, VectorImageError> {
    decode_vector_bounded_with_limits(
        bytes,
        &ImageLimits::default(),
        &VectorLimits::default(),
        None,
    )
}

/// Decodes SVG or SVGZ under injectable inclusive limits.
///
/// The general input gate runs first. SVGZ is then streamed through a one-byte
/// over-limit probe so the actual uncompressed boundary and gzip trailer are
/// both checked before any XML work starts.
///
/// # Errors
///
/// Returns a typed [`VectorImageError`] for every bounded policy or decode failure.
pub fn decode_vector_bounded_with_limits(
    bytes: &[u8],
    image_limits: &ImageLimits,
    vector_limits: &VectorLimits,
    declared: Option<VectorFormat>,
) -> Result<DecodedImage, VectorImageError> {
    let size = bytes.len() as u64;
    if size > image_limits.max_input_bytes {
        return Err(VectorImageError::InputTooLarge {
            size,
            limit: image_limits.max_input_bytes,
        });
    }

    let format = if bytes.starts_with(&[0x1f, 0x8b]) {
        VectorFormat::Svgz
    } else {
        sniff_vector_format(bytes)
            .or(declared)
            .ok_or(VectorImageError::UnsupportedFormat)?
    };
    let xml = match format {
        VectorFormat::Svg => {
            if size > image_limits.max_svg_xml_bytes {
                return Err(VectorImageError::XmlTooLarge {
                    limit: image_limits.max_svg_xml_bytes,
                });
            }
            bytes.to_vec()
        }
        VectorFormat::Svgz => decompress_svgz(bytes, image_limits.max_svg_xml_bytes)?,
    };
    let text = std::str::from_utf8(&xml).map_err(|_| VectorImageError::InvalidUtf8)?;

    validate_xml_structure(&xml, vector_limits.xml)
        .map_err(|source| VectorImageError::UnsafeXml { source })?;
    preflight_svg(text, vector_limits)?;

    let options = Options {
        resources_dir: None,
        image_href_resolver: ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Options::default()
    };
    let tree = Tree::from_str(text, &options).map_err(|error| VectorImageError::ParseFailed {
        detail: error.to_string(),
    })?;
    let output_size = tree.size().to_int_size();
    let width = output_size.width();
    let height = output_size.height();
    if width > image_limits.max_dimension || height > image_limits.max_dimension {
        return Err(VectorImageError::DimensionTooLarge {
            width,
            height,
            limit: image_limits.max_dimension,
        });
    }
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > image_limits.max_pixels {
        return Err(VectorImageError::TooManyPixels {
            pixels,
            limit: image_limits.max_pixels,
        });
    }
    let allocation = pixels.saturating_mul(4);
    if allocation > image_limits.max_allocation_bytes {
        return Err(VectorImageError::AllocationTooLarge {
            bytes: allocation,
            limit: image_limits.max_allocation_bytes,
        });
    }

    let mut pixmap = Pixmap::new(width, height).ok_or(VectorImageError::RenderAllocationFailed)?;
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    let mut rgba = pixmap.take();
    unpremultiply(&mut rgba);
    Ok(DecodedImage::new(width, height, rgba))
}

fn decompress_svgz(bytes: &[u8], limit: u64) -> Result<Vec<u8>, VectorImageError> {
    let capacity = usize::try_from(limit.min(bytes.len() as u64)).unwrap_or(bytes.len());
    let mut output = Vec::with_capacity(capacity);
    let mut decoder = GzDecoder::new(bytes);
    let mut buffer = [0_u8; 8192];
    loop {
        let remaining = limit.saturating_sub(output.len() as u64);
        let request = usize::try_from(remaining.saturating_add(1).min(buffer.len() as u64))
            .unwrap_or(buffer.len());
        match decoder.read(&mut buffer[..request]) {
            Ok(0) => break,
            Ok(read) if read as u64 > remaining => {
                return Err(VectorImageError::XmlTooLarge { limit });
            }
            Ok(read) => output.extend_from_slice(&buffer[..read]),
            Err(error) => return Err(classify_gzip_error(&error)),
        }
    }
    if !decoder.into_inner().is_empty() {
        return Err(VectorImageError::GzipTrailingData);
    }
    Ok(output)
}

fn classify_gzip_error(error: &std::io::Error) -> VectorImageError {
    let message = error.to_string().to_ascii_lowercase();
    if error.kind() == std::io::ErrorKind::UnexpectedEof || message.contains("unexpected end") {
        VectorImageError::GzipTruncated
    } else if message.contains("checksum") || message.contains("crc") {
        VectorImageError::GzipChecksum
    } else {
        VectorImageError::GzipCorrupt
    }
}

fn unpremultiply(rgba: &mut [u8]) {
    let (pixels, remainder) = rgba.as_chunks_mut::<4>();
    debug_assert!(remainder.is_empty());
    for pixel in pixels {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut rest = text.strip_prefix('\u{feff}').unwrap_or(text).trim_start();
    loop {
        if rest.starts_with("<?") {
            let Some(end) = rest.find("?>") else {
                return false;
            };
            rest = rest[end + 2..].trim_start();
        } else if rest.starts_with("<!--") {
            let Some(end) = rest.find("-->") else {
                return false;
            };
            rest = rest[end + 3..].trim_start();
        } else {
            break;
        }
    }
    if [
        "<!doctype",
        "<!entity",
        "<!element",
        "<!attlist",
        "<!notation",
    ]
    .iter()
    .any(|declaration| {
        rest.get(..declaration.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(declaration))
    }) {
        return true;
    }
    rest.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<svg"))
        && rest
            .as_bytes()
            .get(4)
            .is_some_and(|byte| byte.is_ascii_whitespace() || matches!(*byte, b'>' | b'/'))
}

fn preflight_svg(text: &str, limits: &VectorLimits) -> Result<(), VectorImageError> {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    let mut work = WorkTotals::default();
    while let Some(relative) = bytes[cursor..].iter().position(|byte| *byte == b'<') {
        let start = cursor + relative;
        if bytes[start..].starts_with(b"<!--") {
            cursor = skip_markup(bytes, start + 4, b"-->");
            continue;
        }
        if bytes[start..].starts_with(b"<![CDATA[") {
            cursor = skip_markup(bytes, start + 9, b"]]>");
            continue;
        }
        if bytes[start..].starts_with(b"<?") {
            cursor = skip_markup(bytes, start + 2, b"?>");
            continue;
        }
        if bytes.get(start + 1) == Some(&b'/') {
            cursor = tag_close(bytes, start + 2);
            continue;
        }
        let mut at = start + 1;
        skip_space(bytes, &mut at);
        let name_start = at;
        while bytes.get(at).is_some_and(|byte| is_name_byte(*byte)) {
            at += 1;
        }
        let element = local_name(&text[name_start..at]).to_ascii_lowercase();
        inspect_element(&element)?;

        loop {
            skip_space(bytes, &mut at);
            if bytes
                .get(at)
                .is_none_or(|byte| matches!(*byte, b'>' | b'/'))
            {
                cursor = tag_close(bytes, at);
                break;
            }
            let attribute_start = at;
            while bytes.get(at).is_some_and(|byte| is_name_byte(*byte)) {
                at += 1;
            }
            if attribute_start == at {
                cursor = tag_close(bytes, at);
                break;
            }
            let attribute = local_name(&text[attribute_start..at]).to_ascii_lowercase();
            skip_space(bytes, &mut at);
            if bytes.get(at) != Some(&b'=') {
                continue;
            }
            at += 1;
            skip_space(bytes, &mut at);
            let Some(quote @ (b'\'' | b'"')) = bytes.get(at).copied() else {
                cursor = tag_close(bytes, at);
                break;
            };
            at += 1;
            let value_start = at;
            while bytes.get(at).is_some_and(|byte| *byte != quote) {
                at += 1;
            }
            let value = &text[value_start..at];
            at = at.saturating_add(1);
            enforce_work(
                VectorWork::AttributeBytes,
                value.len(),
                limits.max_attribute_bytes,
            )?;
            inspect_attribute(&attribute, value)?;
            work.record(&element, &attribute, value, limits)?;
        }
    }
    Ok(())
}

fn inspect_element(element: &str) -> Result<(), VectorImageError> {
    if matches!(
        element,
        "script"
            | "foreignobject"
            | "animate"
            | "animatemotion"
            | "animatetransform"
            | "set"
            | "discard"
    ) {
        return Err(VectorImageError::ForbiddenElement {
            name: element.to_owned(),
        });
    }
    if element == "style" {
        return Err(VectorImageError::UnsupportedFeature {
            feature: "style sheet",
        });
    }
    if element == "filter" || element.starts_with("fe") {
        return Err(VectorImageError::UnsupportedFeature { feature: "filter" });
    }
    Ok(())
}

#[derive(Default)]
struct WorkTotals {
    path_bytes: usize,
    path_commands: usize,
    transforms: usize,
}

impl WorkTotals {
    fn record(
        &mut self,
        element: &str,
        attribute: &str,
        value: &str,
        limits: &VectorLimits,
    ) -> Result<(), VectorImageError> {
        if element == "path" && attribute == "d" {
            self.path_bytes = self.path_bytes.saturating_add(value.len());
            self.path_commands = self
                .path_commands
                .saturating_add(value.bytes().filter(|byte| is_path_command(*byte)).count());
            enforce_work(
                VectorWork::PathDataBytes,
                self.path_bytes,
                limits.max_path_data_bytes,
            )?;
            enforce_work(
                VectorWork::PathCommands,
                self.path_commands,
                limits.max_path_commands,
            )?;
        }
        if attribute == "transform" {
            self.transforms = self
                .transforms
                .saturating_add(value.bytes().filter(|byte| *byte == b'(').count());
            enforce_work(
                VectorWork::TransformOperations,
                self.transforms,
                limits.max_transform_operations,
            )?;
        }
        Ok(())
    }
}

fn is_path_command(byte: u8) -> bool {
    matches!(
        byte,
        b'M' | b'm'
            | b'Z'
            | b'z'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'Q'
            | b'q'
            | b'T'
            | b't'
            | b'A'
            | b'a'
    )
}

fn inspect_attribute(name: &str, value: &str) -> Result<(), VectorImageError> {
    const ANIMATION_ATTRIBUTES: [&str; 14] = [
        "begin",
        "dur",
        "end",
        "min",
        "max",
        "restart",
        "repeatcount",
        "repeatdur",
        "fill",
        "calcmode",
        "values",
        "keytimes",
        "keysplines",
        "attributename",
    ];
    if name.starts_with("on") || ANIMATION_ATTRIBUTES.contains(&name) && name != "fill" {
        return Err(VectorImageError::ForbiddenAttribute {
            name: name.to_owned(),
        });
    }
    if matches!(name, "href" | "src" | "base") && !value.trim().starts_with('#') {
        return Err(VectorImageError::ExternalReference {
            attribute: name.to_owned(),
        });
    }
    let lower = value.to_ascii_lowercase();
    if lower.contains("@import") || has_external_url(&lower) {
        return Err(VectorImageError::ExternalReference {
            attribute: name.to_owned(),
        });
    }
    if name == "style" && (lower.contains("animation") || lower.contains("transition")) {
        return Err(VectorImageError::ForbiddenAttribute {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn has_external_url(value: &str) -> bool {
    let mut rest = value;
    while let Some(index) = rest.find("url(") {
        rest = &rest[index + 4..];
        let Some(end) = rest.find(')') else {
            return true;
        };
        let target = rest[..end].trim().trim_matches(['\'', '"']);
        if !target.starts_with('#') {
            return true;
        }
        rest = &rest[end + 1..];
    }
    false
}

fn enforce_work(work: VectorWork, observed: usize, limit: usize) -> Result<(), VectorImageError> {
    if observed > limit {
        Err(VectorImageError::WorkLimit {
            work,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-' | b'.')
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
}

fn skip_markup(bytes: &[u8], start: usize, terminator: &[u8]) -> usize {
    bytes[start..]
        .windows(terminator.len())
        .position(|window| window == terminator)
        .map_or(bytes.len(), |relative| start + relative + terminator.len())
}

fn tag_close(bytes: &[u8], mut cursor: usize) -> usize {
    let mut quote = None;
    while let Some(byte) = bytes.get(cursor).copied() {
        match (quote, byte) {
            (Some(expected), current) if expected == current => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return cursor + 1,
            _ => {}
        }
        cursor += 1;
    }
    bytes.len()
}
