//! Emit the tiny authored raster corpus used by `tools/fixture_corpus.py`.

use std::{env, fs, io::Cursor, path::Path};

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};

fn encode(format: ImageFormat, image: &DynamicImage) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, format)
        .unwrap_or_else(|error| panic!("encode {format:?}: {error}"));
    cursor.into_inner()
}

fn dxt1_red_dds() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(136);
    bytes.extend_from_slice(b"DDS ");
    bytes.extend_from_slice(&124u32.to_le_bytes());
    bytes.extend_from_slice(&0x0008_1007u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&[0; 44]);
    bytes.extend_from_slice(&32u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(b"DXT1");
    bytes.extend_from_slice(&[0; 16]);
    bytes.extend_from_slice(&0x1000u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 20]);
    bytes.extend_from_slice(&0xF800u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes
}

fn main() {
    let output = env::args_os()
        .nth(1)
        .expect("usage: cargo run --example fixture_rasters -- OUTPUT");
    let output = Path::new(&output);
    fs::create_dir_all(output).expect("create raster output directory");

    let standard = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([220, 40, 20, 255])));
    let jpeg =
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([20, 180, 60])));
    let hdr = DynamicImage::ImageRgb32F(image::Rgb32FImage::from_pixel(
        2,
        2,
        image::Rgb([0.25, 0.5, 1.0]),
    ));
    let exr = DynamicImage::ImageRgba32F(image::Rgba32FImage::from_pixel(
        2,
        2,
        image::Rgba([0.25, 0.5, 1.0, 1.0]),
    ));
    let farbfeld =
        DynamicImage::ImageRgba16(ImageBuffer::from_pixel(2, 2, Rgba([65535u16, 0, 0, 65535])));
    let fixtures = [
        ("minimal.png", encode(ImageFormat::Png, &standard)),
        ("minimal.jpg", encode(ImageFormat::Jpeg, &jpeg)),
        ("minimal.gif", encode(ImageFormat::Gif, &standard)),
        ("minimal.webp", encode(ImageFormat::WebP, &standard)),
        ("minimal.bmp", encode(ImageFormat::Bmp, &standard)),
        ("minimal.ico", encode(ImageFormat::Ico, &standard)),
        ("minimal.tiff", encode(ImageFormat::Tiff, &standard)),
        ("minimal.ppm", encode(ImageFormat::Pnm, &jpeg)),
        ("minimal.tga", encode(ImageFormat::Tga, &standard)),
        ("minimal.qoi", encode(ImageFormat::Qoi, &standard)),
        ("minimal.dds", dxt1_red_dds()),
        ("minimal.hdr", encode(ImageFormat::Hdr, &hdr)),
        ("minimal.exr", encode(ImageFormat::OpenExr, &exr)),
        ("minimal.ff", encode(ImageFormat::Farbfeld, &farbfeld)),
    ];
    for (name, bytes) in fixtures {
        fs::write(output.join(name), bytes).unwrap_or_else(|error| panic!("write {name}: {error}"));
    }
}
