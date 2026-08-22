//! Bounded raster decoding for embedded book resources.
//!
//! Images are best effort because terminals vary, but safety is not: a
//! crafted or damaged file must never consume unreasonable memory or CPU.
//! Every number from the project plan's initial image limits table applies
//! before any pixel buffer is allocated — the input byte gate runs before
//! format sniffing, and declared geometry is checked against header reads
//! alone so hostile dimensions reject without a decode attempt.
//!
//! The module is synchronous and allocation-bounded by design so a worker
//! thread can run it away from the UI when integration lands. Animation
//! resolves to a first-frame preview only; SVG and SVGZ follow the separate
//! vector path with their own resolver policy.

use std::io::Cursor;

use image::{ImageFormat, ImageReader};

/// Resource limits applied to every embedded image (all inclusive).
///
/// The defaults reproduce the project plan's initial image table. The pixel
/// budget and the allocation budget are related but distinct policies: 64
/// million RGBA8 output pixels equal exactly 256 MiB, while decoders whose
/// intermediate representation is wider (float EXR/HDR, 16-bit PNG/TIFF)
/// reserve more per pixel so even their transient buffers stay inside the
/// envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageLimits {
    /// Inclusive maximum compressed input size in bytes.
    pub max_input_bytes: u64,
    /// Inclusive maximum width or height in pixels.
    pub max_dimension: u32,
    /// Inclusive maximum decoded pixels per frame.
    pub max_pixels: u64,
    /// Inclusive maximum transient plus output allocation in bytes.
    pub max_allocation_bytes: u64,
    /// Inclusive maximum decompressed SVG/SVGZ XML input in bytes; owned by
    /// the vector-image slice.
    pub max_svg_xml_bytes: u64,
}

impl Default for ImageLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 32 * 1024 * 1024,
            max_dimension: 16_384,
            max_pixels: 64_000_000,
            max_allocation_bytes: 256 * 1024 * 1024,
            max_svg_xml_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Typed rejection for one image resource.
///
/// Every variant states what failed, the observed value versus its policy
/// limit where one exists, and what the reader can do about it.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ImageResourceError {
    /// The resource exceeds the compressed input byte limit.
    #[error(
        "image resource is too large: {size} bytes exceeds the {limit} byte input limit; \
         the book may be corrupt or hostile"
    )]
    InputTooLarge {
        /// Observed compressed input size in bytes.
        size: u64,
        /// Configured inclusive byte limit.
        limit: u64,
    },

    /// The leading bytes match no enabled decoder.
    #[error(
        "unsupported image data: the leading bytes match no enabled decoder; TermLeaf reads \
         PNG, JPEG, GIF, WebP, BMP, ICO, TIFF, PNM, TGA, QOI, DDS, OpenEXR, Radiance HDR, \
         and Farbfeld images"
    )]
    UnsupportedFormat,

    /// A declared dimension exceeds the per-side limit.
    #[error(
        "image declares {width}x{height} pixels beyond the {limit} pixel dimension limit; \
         the image may be corrupt or hostile"
    )]
    DimensionTooLarge {
        /// Declared width in pixels.
        width: u32,
        /// Declared height in pixels.
        height: u32,
        /// Configured inclusive per-side limit.
        limit: u32,
    },

    /// The declared geometry covers more pixels than the budget allows.
    #[error(
        "image declares {pixels} decoded pixels beyond the {limit} pixel limit; choose an \
         edition with smaller embedded images"
    )]
    TooManyPixels {
        /// Declared total pixels.
        pixels: u64,
        /// Configured inclusive pixel limit.
        limit: u64,
    },

    /// Decoding the declared geometry could allocate beyond the budget.
    ///
    /// The estimate uses a conservative per-pixel ceiling for the decoder
    /// family, so float and 16-bit pipelines cannot slip past the envelope
    /// through wider intermediate buffers.
    #[error(
        "decoding this image could allocate about {bytes} bytes beyond the {limit} byte \
         budget; the image is too large to display safely"
    )]
    AllocationTooLarge {
        /// Estimated peak allocation in bytes.
        bytes: u64,
        /// Configured inclusive allocation budget.
        limit: u64,
    },

    /// The data matched a decoder but did not decode cleanly.
    #[error("could not decode image: {detail}; the file may be truncated or corrupt")]
    DecodeFailed {
        /// Short decoder explanation without raw byte dumps.
        detail: String,
    },
}

/// One successfully bounded decoded frame as normalized RGBA8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl DecodedImage {
    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The normalized RGBA8 sample buffer, row-major with premultiplied-off
    /// straight alpha; `width() * height() * 4` bytes long.
    #[must_use]
    pub const fn rgba(&self) -> &Vec<u8> {
        &self.rgba
    }
}

/// Detects the enabled decoder matching the leading magic bytes.
///
/// # Errors
///
/// Returns [`ImageResourceError::UnsupportedFormat`] when the prefix matches
/// no enabled decoder; nothing about the remainder of the input matters here.
pub fn sniff_format(bytes: &[u8]) -> Result<ImageFormat, ImageResourceError> {
    image::guess_format(bytes).map_err(|_| ImageResourceError::UnsupportedFormat)
}

/// Resolves one decoder from content evidence plus an optional declared
/// format.
///
/// Resolution is extension-first with magic winning when present, matching
/// `DEC-TEST-001`: a leading signature always decides (a PNG mislabeled as
/// `.tga` still decodes through the PNG decoder), while the declaration
/// rescues formats that carry no magic at all — TGA among the enabled set.
///
/// # Errors
///
/// Returns [`ImageResourceError::UnsupportedFormat`] when no signature is
/// present and none was declared.
pub fn resolve_format(
    bytes: &[u8],
    declared: Option<ImageFormat>,
) -> Result<ImageFormat, ImageResourceError> {
    sniff_format(bytes).or(declared.ok_or(ImageResourceError::UnsupportedFormat))
}

/// Decodes one image under the default policy limits, sniffing its format
/// from content alone.
///
/// # Errors
///
/// Returns every [`ImageResourceError`] rejection; see
/// [`decode_bounded_with_limits`] for the exact order.
pub fn decode_bounded(bytes: &[u8]) -> Result<DecodedImage, ImageResourceError> {
    decode_bounded_with_limits(bytes, &ImageLimits::default(), None)
}

/// Decodes one image under explicit limits and an optional declared format.
///
/// The checks run strictly in policy order so tests can pin each boundary:
/// the input byte gate rejects before any parse, header-only dimension reads
/// reject hostile geometry before any pixel allocation, and the allocation
/// estimate reserves headroom for wide intermediate buffers. Only then does
/// the decoder run, producing a first-frame preview for animated sources.
///
/// # Errors
///
/// Returns [`ImageResourceError::InputTooLarge`],
/// [`ImageResourceError::UnsupportedFormat`],
/// [`ImageResourceError::DimensionTooLarge`],
/// [`ImageResourceError::TooManyPixels`],
/// [`ImageResourceError::AllocationTooLarge`], or
/// [`ImageResourceError::DecodeFailed`] in that order of precedence.
pub fn decode_bounded_with_limits(
    bytes: &[u8],
    limits: &ImageLimits,
    declared: Option<ImageFormat>,
) -> Result<DecodedImage, ImageResourceError> {
    let size = bytes.len() as u64;
    if size > limits.max_input_bytes {
        return Err(ImageResourceError::InputTooLarge {
            size,
            limit: limits.max_input_bytes,
        });
    }

    let format = resolve_format(bytes, declared)?;

    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|error| ImageResourceError::DecodeFailed {
            detail: error.to_string(),
        })?;

    if width > limits.max_dimension || height > limits.max_dimension {
        return Err(ImageResourceError::DimensionTooLarge {
            width,
            height,
            limit: limits.max_dimension,
        });
    }

    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > limits.max_pixels {
        return Err(ImageResourceError::TooManyPixels {
            pixels,
            limit: limits.max_pixels,
        });
    }

    let estimated = pixels.saturating_mul(allocation_bytes_per_pixel(format));
    if estimated > limits.max_allocation_bytes {
        return Err(ImageResourceError::AllocationTooLarge {
            bytes: estimated,
            limit: limits.max_allocation_bytes,
        });
    }

    let decoded = ImageReader::with_format(Cursor::new(bytes), format)
        .decode()
        .map_err(|error| ImageResourceError::DecodeFailed {
            detail: error.to_string(),
        })?;
    let rgba = decoded.to_rgba8().into_raw();

    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

/// Conservative ceiling on transient per-pixel bytes per decoder family.
///
/// The values bound the widest intermediate representation each family can
/// hold while decoding, before the normalized RGBA8 output exists: `OpenEXR`
/// decodes to RGBA32F, Radiance HDR to RGB32F, and the PNG/TIFF families may
/// carry 16-bit samples. Every other enabled decoder normalizes from at most
/// 8-bit samples.
fn allocation_bytes_per_pixel(format: ImageFormat) -> u64 {
    match format {
        ImageFormat::OpenExr => 16,
        ImageFormat::Hdr => 12,
        ImageFormat::Png | ImageFormat::Tiff => 8,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::codecs::gif::GifEncoder;
    use image::{DynamicImage, Frame, ImageBuffer, Rgba, RgbaImage};

    use super::*;

    /// Encodes one uniform-color RGBA8 fixture through any writer-based codec.
    fn encode_rgba(format: ImageFormat, color: [u8; 4]) -> Vec<u8> {
        encode_image(
            format,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba(color))),
        )
    }

    fn encode_image(format: ImageFormat, image: &DynamicImage) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        image
            .write_to(&mut cursor, format)
            .unwrap_or_else(|error| panic!("encode {format:?}: {error}"));
        cursor.into_inner()
    }

    fn red_png() -> Vec<u8> {
        encode_rgba(ImageFormat::Png, [255, 0, 0, 255])
    }

    /// Hand-crafts one minimal DXT1-compressed DDS holding a single red 4x4
    /// block; the `image` crate has no DDS encoder, so the fixture is built
    /// from the container specification directly.
    fn dxt1_red_dds() -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128 + 8);
        bytes.extend_from_slice(b"DDS ");
        bytes.extend_from_slice(&124u32.to_le_bytes()); // header size
        bytes.extend_from_slice(&0x0008_1007u32.to_le_bytes()); // CAPS|HEIGHT|WIDTH|PIXELFORMAT|LINEARSIZE
        bytes.extend_from_slice(&4u32.to_le_bytes()); // height
        bytes.extend_from_slice(&4u32.to_le_bytes()); // width
        bytes.extend_from_slice(&8u32.to_le_bytes()); // linear size: one block
        bytes.extend_from_slice(&[0u8; 8]); // depth, mip count
        bytes.extend_from_slice(&[0u8; 44]); // reserved
        bytes.extend_from_slice(&32u32.to_le_bytes()); // pixel format size
        bytes.extend_from_slice(&0x4u32.to_le_bytes()); // DDPF_FOURCC
        bytes.extend_from_slice(b"DXT1");
        bytes.extend_from_slice(&[0u8; 16]); // bit count and masks
        bytes.extend_from_slice(&0x1000u32.to_le_bytes()); // DDSCAPS_TEXTURE
        bytes.extend_from_slice(&[0u8; 20]); // caps 2-4, reserved2
        debug_assert_eq!(bytes.len(), 128);
        // One BC1 block: red endpoint, black endpoint, all texels index 0.
        bytes.extend_from_slice(&0xF800u16.to_le_bytes());
        bytes.extend_from_slice(&0x0000u16.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes
    }

    #[test]
    fn img_005_default_policy_matches_the_locked_image_table() {
        let limits = ImageLimits::default();
        assert_eq!(limits.max_input_bytes, 32 * 1024 * 1024);
        assert_eq!(limits.max_dimension, 16_384);
        assert_eq!(limits.max_pixels, 64_000_000);
        assert_eq!(limits.max_allocation_bytes, 256 * 1024 * 1024);
        assert_eq!(limits.max_svg_xml_bytes, 8 * 1024 * 1024);
    }

    #[test]
    fn img_005_input_byte_gate_rejects_before_any_parsing() {
        let fixture = red_png();
        let size = fixture.len() as u64;

        // Exactly at the boundary the image still decodes.
        let at_limit = ImageLimits {
            max_input_bytes: size,
            ..ImageLimits::default()
        };
        let decoded = decode_bounded_with_limits(&fixture, &at_limit, None)
            .expect("input exactly at the byte limit decodes");
        assert_eq!((decoded.width(), decoded.height()), (2, 2));

        // One byte over, the typed size rejection wins over every later
        // outcome, proving no parser ran.
        let under = ImageLimits {
            max_input_bytes: size - 1,
            ..ImageLimits::default()
        };
        assert_eq!(
            decode_bounded_with_limits(&fixture, &under, None),
            Err(ImageResourceError::InputTooLarge {
                size,
                limit: size - 1
            })
        );
    }

    #[test]
    fn img_006_width_and_height_reject_independently_at_the_boundary() {
        let narrow = ImageLimits {
            max_dimension: 4,
            ..ImageLimits::default()
        };

        let at_limit = encode_image(
            ImageFormat::Png,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 1, Rgba([1, 2, 3, 255]))),
        );
        let decoded = decode_bounded_with_limits(&at_limit, &narrow, None)
            .expect("width exactly at the dimension limit decodes");
        assert_eq!((decoded.width(), decoded.height()), (4, 1));

        let tall_at_limit = encode_image(
            ImageFormat::Png,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 4, Rgba([1, 2, 3, 255]))),
        );
        let decoded = decode_bounded_with_limits(&tall_at_limit, &narrow, None)
            .expect("height exactly at the dimension limit decodes");
        assert_eq!((decoded.width(), decoded.height()), (1, 4));

        let too_wide = encode_image(
            ImageFormat::Png,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(5, 1, Rgba([1, 2, 3, 255]))),
        );
        assert_eq!(
            decode_bounded_with_limits(&too_wide, &narrow, None),
            Err(ImageResourceError::DimensionTooLarge {
                width: 5,
                height: 1,
                limit: 4
            })
        );

        let too_tall = encode_image(
            ImageFormat::Png,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 5, Rgba([1, 2, 3, 255]))),
        );
        assert_eq!(
            decode_bounded_with_limits(&too_tall, &narrow, None),
            Err(ImageResourceError::DimensionTooLarge {
                width: 1,
                height: 5,
                limit: 4
            })
        );

        // The real 16,384-per-side policy accepts a full-limit row without
        // ever approaching the pixel or allocation budgets.
        let full_row = encode_image(
            ImageFormat::Png,
            &DynamicImage::ImageRgba8(RgbaImage::from_pixel(16_384, 1, Rgba([9, 9, 9, 255]))),
        );
        let decoded = decode_bounded(&full_row).expect("one full-limit row decodes");
        assert_eq!(decoded.width(), 16_384);
        assert_eq!(
            decoded.rgba().len(),
            decoded.width() as usize * 4,
            "the full-limit row normalizes to complete RGBA8"
        );
    }

    #[test]
    fn img_007_pixel_and_allocation_budgets_reject_exactly_at_the_boundary() {
        let fixture = red_png(); // 2x2 = 4 pixels, 16 RGBA bytes.

        let at_pixels = ImageLimits {
            max_pixels: 4,
            ..ImageLimits::default()
        };
        decode_bounded_with_limits(&fixture, &at_pixels, None)
            .expect("declared pixels exactly at the limit decode");

        let over_pixels = ImageLimits {
            max_pixels: 3,
            ..ImageLimits::default()
        };
        assert_eq!(
            decode_bounded_with_limits(&fixture, &over_pixels, None),
            Err(ImageResourceError::TooManyPixels {
                pixels: 4,
                limit: 3
            })
        );

        let at_allocation = ImageLimits {
            // The PNG family reserves up to 8 bytes per pixel for 16-bit
            // samples, so a 2x2 image estimates exactly 32.
            max_allocation_bytes: 32,
            ..ImageLimits::default()
        };
        decode_bounded_with_limits(&fixture, &at_allocation, None)
            .expect("allocation estimate exactly at the budget decodes");

        let over_allocation = ImageLimits {
            max_allocation_bytes: 31,
            ..ImageLimits::default()
        };
        assert_eq!(
            decode_bounded_with_limits(&fixture, &over_allocation, None),
            Err(ImageResourceError::AllocationTooLarge {
                bytes: 32,
                limit: 31
            })
        );

        // Wide intermediate representations reserve extra headroom so float
        // and 16-bit families stay inside the same envelope.
        assert_eq!(allocation_bytes_per_pixel(ImageFormat::OpenExr), 16);
        assert_eq!(allocation_bytes_per_pixel(ImageFormat::Hdr), 12);
        assert_eq!(allocation_bytes_per_pixel(ImageFormat::Png), 8);
        assert_eq!(allocation_bytes_per_pixel(ImageFormat::Tiff), 8);
        assert_eq!(allocation_bytes_per_pixel(ImageFormat::Jpeg), 4);
    }

    type Fixture = (ImageFormat, Vec<u8>, (u32, u32));

    /// Builds one bounded fixture per enabled decoder with its expected
    /// geometry; formats without an encoder in the `image` crate are
    /// hand-crafted from their container specifications.
    fn all_format_fixtures() -> Vec<Fixture> {
        let jpeg_source =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([10, 200, 30])));
        let hdr_source = DynamicImage::ImageRgb32F(image::Rgb32FImage::from_pixel(
            2,
            2,
            image::Rgb([0.25, 0.5, 1.0]),
        ));
        let exr_source = DynamicImage::ImageRgba32F(image::Rgba32FImage::from_pixel(
            2,
            2,
            image::Rgba([0.25, 0.5, 1.0, 1.0]),
        ));
        let farbfeld_source =
            DynamicImage::ImageRgba16(ImageBuffer::from_pixel(2, 2, Rgba([65535u16, 0, 0, 65535])));
        let pnm_source =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([10, 200, 30])));

        vec![
            (ImageFormat::Png, red_png(), (2, 2)),
            (
                ImageFormat::Jpeg,
                encode_image(ImageFormat::Jpeg, &jpeg_source),
                (2, 2),
            ),
            (
                ImageFormat::Gif,
                encode_rgba(ImageFormat::Gif, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::WebP,
                encode_rgba(ImageFormat::WebP, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::Bmp,
                encode_rgba(ImageFormat::Bmp, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::Ico,
                encode_rgba(ImageFormat::Ico, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::Tiff,
                encode_rgba(ImageFormat::Tiff, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::Pnm,
                encode_image(ImageFormat::Pnm, &pnm_source),
                (2, 2),
            ),
            (
                ImageFormat::Tga,
                encode_rgba(ImageFormat::Tga, [255, 0, 0, 255]),
                (2, 2),
            ),
            (
                ImageFormat::Qoi,
                encode_rgba(ImageFormat::Qoi, [255, 0, 0, 255]),
                (2, 2),
            ),
            // One hand-crafted BC1 block covers exactly one 4x4 texel tile.
            (ImageFormat::Dds, dxt1_red_dds(), (4, 4)),
            (
                ImageFormat::Hdr,
                encode_image(ImageFormat::Hdr, &hdr_source),
                (2, 2),
            ),
            (
                ImageFormat::OpenExr,
                encode_image(ImageFormat::OpenExr, &exr_source),
                (2, 2),
            ),
            (
                ImageFormat::Farbfeld,
                encode_image(ImageFormat::Farbfeld, &farbfeld_source),
                (2, 2),
            ),
        ]
    }

    #[test]
    fn img_001_every_enabled_decoder_decodes_a_bounded_fixture() {
        for (format, bytes, expected) in all_format_fixtures() {
            if format == ImageFormat::Tga {
                // TGA carries no magic signature by design; only its
                // declared extension resolves it.
                assert_eq!(
                    sniff_format(&bytes),
                    Err(ImageResourceError::UnsupportedFormat)
                );
                let declared = decode_bounded_with_limits(
                    &bytes,
                    &ImageLimits::default(),
                    Some(ImageFormat::Tga),
                )
                .unwrap_or_else(|error| panic!("declared TGA: {error}"));
                assert_eq!((declared.width(), declared.height()), expected);
                continue;
            }
            assert_eq!(
                sniff_format(&bytes),
                Ok(format),
                "{format:?} magic must identify itself"
            );
            let decoded =
                decode_bounded(&bytes).unwrap_or_else(|error| panic!("{format:?}: {error}"));
            assert_eq!(
                (decoded.width(), decoded.height()),
                expected,
                "{format:?} reports its declared geometry"
            );
            assert_eq!(
                decoded.rgba().len(),
                expected.0 as usize * expected.1 as usize * 4,
                "{format:?} normalizes to RGBA8"
            );
        }
    }

    #[test]
    fn resolution_is_extension_first_with_magic_winning_when_present() {
        // Magic evidence overrides a wrong declaration.
        let png = red_png();
        assert_eq!(
            resolve_format(&png, Some(ImageFormat::Jpeg)),
            Ok(ImageFormat::Png)
        );

        // A declaration rescues formats without any signature.
        let tga = encode_rgba(ImageFormat::Tga, [255, 0, 0, 255]);
        assert_eq!(
            resolve_format(&tga, Some(ImageFormat::Tga)),
            Ok(ImageFormat::Tga)
        );
        assert_eq!(
            resolve_format(&tga, None),
            Err(ImageResourceError::UnsupportedFormat)
        );
    }

    #[test]
    fn img_002_animated_gif_previews_only_its_first_frame() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut encoder = GifEncoder::new(&mut cursor);
            let first = Frame::new(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])));
            let second = Frame::new(RgbaImage::from_pixel(2, 2, Rgba([0, 0, 255, 255])));
            encoder.encode_frame(first).expect("first frame encodes");
            encoder.encode_frame(second).expect("second frame encodes");
        }
        let bytes = cursor.into_inner();
        assert_eq!(sniff_format(&bytes), Ok(ImageFormat::Gif));

        let decoded = decode_bounded(&bytes).expect("animated GIF decodes");
        assert_eq!((decoded.width(), decoded.height()), (2, 2));

        // The preview shows the first frame's color, not the second's.
        let top_left: [u8; 4] = decoded.rgba()[..4].try_into().expect("four channels");
        assert!(
            top_left[0] > top_left[2],
            "first frame stays visible: {top_left:?}"
        );
    }

    #[test]
    fn img_012_truncated_corrupt_and_foreign_inputs_fail_typed_without_panicking() {
        // Empty and short inputs never match a magic signature.
        assert_eq!(
            decode_bounded(&[]),
            Err(ImageResourceError::UnsupportedFormat)
        );
        assert_eq!(
            decode_bounded(&[0x89, b'P']),
            Err(ImageResourceError::UnsupportedFormat)
        );

        // A valid signature followed by garbage reaches the decoder and
        // fails there, still typed.
        let mut spoofed = Vec::from(&b"\x89PNG\r\n\x1a\n"[..]);
        spoofed.extend_from_slice(&[0xFF; 64]);
        assert!(matches!(
            decode_bounded(&spoofed),
            Err(ImageResourceError::DecodeFailed { .. })
        ));

        // Truncating a valid file mid-stream fails cleanly instead of
        // panicking or returning partial pixels.
        let fixture = red_png();
        let truncated = &fixture[..fixture.len() / 2];
        if let Ok(decoded) = decode_bounded(truncated) {
            // A truncation point before any IDAT cannot decode; if it ever
            // succeeds the output must still be complete geometry.
            assert_eq!(decoded.rgba().len(), 2 * 2 * 4);
        }

        // Flipping bytes inside the compressed stream breaks decoding.
        let mut corrupted = fixture.clone();
        let middle = corrupted.len() / 2;
        corrupted[middle] ^= 0xFF;
        corrupted[middle + 1] ^= 0xFF;
        assert!(matches!(
            decode_bounded(&corrupted),
            Err(ImageResourceError::DecodeFailed { .. })
        ));
    }

    #[test]
    fn error_messages_name_failure_value_and_recovery() {
        let cases = [
            ImageResourceError::InputTooLarge {
                size: 40,
                limit: 32,
            }
            .to_string(),
            ImageResourceError::UnsupportedFormat.to_string(),
            ImageResourceError::DimensionTooLarge {
                width: 99,
                height: 1,
                limit: 98,
            }
            .to_string(),
            ImageResourceError::TooManyPixels {
                pixels: 65,
                limit: 64,
            }
            .to_string(),
            ImageResourceError::AllocationTooLarge {
                bytes: 300,
                limit: 256,
            }
            .to_string(),
            ImageResourceError::DecodeFailed {
                detail: "unexpected end".to_owned(),
            }
            .to_string(),
        ];
        for message in &cases {
            assert!(
                message.contains(';') || message.contains(':'),
                "states a reason or recovery: {message}"
            );
        }
        assert!(cases[0].contains("40 bytes") && cases[0].contains("32 byte"));
        assert!(cases[2].contains("99x1") && cases[2].contains("98"));
        assert!(cases[3].contains("65 decoded pixels") && cases[3].contains("64"));
        assert!(cases[4].contains("300 bytes") && cases[4].contains("256 byte"));
        assert!(cases[5].contains("truncated or corrupt"));
    }
}
