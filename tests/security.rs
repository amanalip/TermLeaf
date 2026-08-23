//! Executable structured-document and archive security profile.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use termleaf::document::{
    ArchiveError, ArchiveLimits, EpubSnapshot, ImageLimits, ImageResourceError, PreflightedArchive,
    VectorFormat, VectorImageError, VectorLimits, VectorWork, XmlLimits, XmlStructureError,
    checked_expansion_total, decode_vector_bounded, decode_vector_bounded_with_limits,
    validate_xml_structure,
};

const CONTROL_PATHS: [&str; 4] = [
    "META-INF/container.xml",
    "OEBPS/content.opf",
    "OEBPS/toc.ncx",
    "OEBPS/navigation.xhtml",
];

fn nested(depth: usize) -> Vec<u8> {
    let mut xml = String::new();
    for _ in 0..depth {
        xml.push_str("<n>");
    }
    for _ in 0..depth {
        xml.push_str("</n>");
    }
    xml.into_bytes()
}

fn wide(nodes: usize) -> Vec<u8> {
    "<n/>".repeat(nodes).into_bytes()
}

fn archive_bytes(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, body) in members {
            writer.start_file(*name, options).expect("start member");
            writer.write_all(body).expect("write member");
        }
        writer.finish().expect("finish archive");
    }
    output.into_inner()
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("compress SVGZ fixture");
    encoder.finish().expect("finish SVGZ fixture")
}

fn svg_with_exact_size(size: usize) -> Vec<u8> {
    const PREFIX: &str = "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><!--";
    const SUFFIX: &str = "--><rect width='1' height='1' fill='#123456'/></svg>";
    assert!(size >= PREFIX.len() + SUFFIX.len());
    format!(
        "{PREFIX}{}{SUFFIX}",
        "x".repeat(size - PREFIX.len() - SUFFIX.len())
    )
    .into_bytes()
}

fn small_image_limits() -> ImageLimits {
    ImageLimits {
        max_input_bytes: 64 * 1024,
        max_dimension: 64,
        max_pixels: 4096,
        max_allocation_bytes: 16 * 1024,
        max_svg_xml_bytes: 64 * 1024,
    }
}

#[test]
fn img_003_safe_svg_and_svgz_render_static_normalized_rgba() {
    let svg = br"<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><rect width='2' height='1' fill='#ff0000' fill-opacity='.5'/></svg>";
    for bytes in [svg.to_vec(), gzip(svg)] {
        let decoded = decode_vector_bounded(&bytes).expect("safe static vector renders");
        assert_eq!((decoded.width(), decoded.height()), (2, 1));
        assert_eq!(decoded.rgba().len(), 8);
        assert!(decoded.rgba()[0] >= 254, "RGBA is straight-alpha red");
        assert_eq!(decoded.rgba()[1], 0);
        assert!((127..=128).contains(&decoded.rgba()[3]));
    }

    let dispatched = termleaf::document::image::decode_bounded(svg)
        .expect("general image dispatch recognizes SVG");
    assert_eq!((dispatched.width(), dispatched.height()), (2, 1));

    for unsafe_svg in [
        "<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>",
        "<svg xmlns='http://www.w3.org/2000/svg'><animate attributeName='x'/></svg>",
        "<svg xmlns='http://www.w3.org/2000/svg' onload='run()'/>",
    ] {
        assert!(matches!(
            decode_vector_bounded(unsafe_svg.as_bytes()),
            Err(VectorImageError::ForbiddenElement { .. }
                | VectorImageError::ForbiddenAttribute { .. })
        ));
    }
    assert!(matches!(
        decode_vector_bounded(
            b"<!DOCTYPE svg [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><svg xmlns='http://www.w3.org/2000/svg'/>"
        ),
        Err(VectorImageError::UnsafeXml {
            source: XmlStructureError::DtdDeclaration
        })
    ));
}

#[test]
fn img_004_every_external_reference_class_rejects_before_resvg() {
    let references = [
        "https://example.invalid/image.png",
        "//example.invalid/image.png",
        "file:///etc/passwd",
        "/etc/passwd",
        "../outside.png",
        "../../outside.png",
        "/dev/null",
        r"\\.\PhysicalDrive0",
        "NUL",
        "data:image/png;base64,iVBORw0KGgo=",
    ];
    for reference in references {
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><image href='{reference}'/></svg>"
        );
        assert_eq!(
            decode_vector_bounded(svg.as_bytes()),
            Err(VectorImageError::ExternalReference {
                attribute: "href".to_owned()
            }),
            "reference must not reach the disabled resvg resolver: {reference}"
        );
    }

    let paint = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1' fill='url(file:///etc/passwd)'/></svg>";
    assert!(matches!(
        decode_vector_bounded(paint),
        Err(VectorImageError::ExternalReference { .. })
    ));
    let internal = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><defs><linearGradient id='g'/></defs><rect width='1' height='1' fill='url(#g)'/></svg>";
    decode_vector_bounded(internal).expect("document-local fragment references remain safe");
}

#[test]
fn img_005_svg_input_and_actual_xml_boundaries_are_inclusive() {
    let xml = svg_with_exact_size(1024);
    let compressed = gzip(&xml);
    let vector = VectorLimits::default();
    let at = ImageLimits {
        max_input_bytes: compressed.len() as u64,
        max_svg_xml_bytes: xml.len() as u64,
        ..small_image_limits()
    };
    decode_vector_bounded_with_limits(&compressed, &at, &vector, None)
        .expect("compressed and actual XML exact boundaries are inclusive");

    let compressed_under = ImageLimits {
        max_input_bytes: compressed.len() as u64 - 1,
        ..at
    };
    assert_eq!(
        decode_vector_bounded_with_limits(&compressed, &compressed_under, &vector, None),
        Err(VectorImageError::InputTooLarge {
            size: compressed.len() as u64,
            limit: compressed.len() as u64 - 1
        })
    );

    let xml_under = ImageLimits {
        max_svg_xml_bytes: xml.len() as u64 - 1,
        ..at
    };
    assert_eq!(
        decode_vector_bounded_with_limits(&compressed, &xml_under, &vector, None),
        Err(VectorImageError::XmlTooLarge {
            limit: xml.len() as u64 - 1
        })
    );
    assert_eq!(
        decode_vector_bounded_with_limits(
            &xml,
            &ImageLimits {
                max_input_bytes: xml.len() as u64,
                ..xml_under
            },
            &vector,
            Some(VectorFormat::Svg)
        ),
        Err(VectorImageError::XmlTooLarge {
            limit: xml.len() as u64 - 1
        })
    );
}

#[test]
fn img_015_svgz_stream_failures_are_distinct_and_bounded() {
    let xml = svg_with_exact_size(4096);
    let valid = gzip(&xml);

    let truncated = &valid[..valid.len() - 3];
    assert_eq!(
        decode_vector_bounded(truncated),
        Err(VectorImageError::GzipTruncated)
    );

    let mut bad_checksum = valid.clone();
    let trailer = bad_checksum.len() - 8;
    bad_checksum[trailer] ^= 0xff;
    assert_eq!(
        decode_vector_bounded(&bad_checksum),
        Err(VectorImageError::GzipChecksum)
    );

    let mut corrupt = valid.clone();
    corrupt[3] = 0xe0;
    assert_eq!(
        decode_vector_bounded(&corrupt),
        Err(VectorImageError::GzipCorrupt)
    );

    let mut concatenated = valid.clone();
    concatenated.extend_from_slice(&gzip(b"ignored second member"));
    assert_eq!(
        decode_vector_bounded(&concatenated),
        Err(VectorImageError::GzipTrailingData)
    );

    let mut false_size = valid.clone();
    let isize = false_size.len() - 4;
    false_size[isize] ^= 1;
    assert_eq!(
        decode_vector_bounded(&false_size),
        Err(VectorImageError::GzipChecksum)
    );

    let limits = ImageLimits {
        max_svg_xml_bytes: 128,
        ..small_image_limits()
    };
    assert_eq!(
        decode_vector_bounded_with_limits(&valid, &limits, &VectorLimits::default(), None),
        Err(VectorImageError::XmlTooLarge { limit: 128 })
    );
}

#[test]
fn img_016_structure_work_geometry_and_allocation_boundaries_are_typed() {
    let image = small_image_limits();
    let vector = VectorLimits {
        xml: XmlLimits {
            max_depth: 2,
            max_nodes: 3,
        },
        max_path_data_bytes: 32,
        max_path_commands: 2,
        max_transform_operations: 1,
        max_attribute_bytes: 64,
    };
    let at_structure = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><g/><path d='M0 0L1 1'/></svg>";
    decode_vector_bounded_with_limits(at_structure, &image, &vector, Some(VectorFormat::Svg))
        .expect("exact depth, nodes, path commands, and transform limits accept");

    let too_deep =
        b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><g><g/></g></svg>";
    assert!(matches!(
        decode_vector_bounded_with_limits(too_deep, &image, &vector, Some(VectorFormat::Svg)),
        Err(VectorImageError::UnsafeXml {
            source: XmlStructureError::TooDeep { depth: 3, limit: 2 }
        })
    ));
    let too_many_nodes =
        b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><g/><g/><g/></svg>";
    assert!(matches!(
        decode_vector_bounded_with_limits(too_many_nodes, &image, &vector, Some(VectorFormat::Svg)),
        Err(VectorImageError::UnsafeXml {
            source: XmlStructureError::TooManyNodes { nodes: 4, limit: 3 }
        })
    ));
    let path = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><path d='M0 0L1 1L2 2'/></svg>";
    assert_eq!(
        decode_vector_bounded_with_limits(path, &image, &vector, Some(VectorFormat::Svg)),
        Err(VectorImageError::WorkLimit {
            work: VectorWork::PathCommands,
            observed: 3,
            limit: 2
        })
    );
    let one_transform = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><g transform='translate(1 1)'/></svg>";
    decode_vector_bounded_with_limits(one_transform, &image, &vector, Some(VectorFormat::Svg))
        .expect("one transform operation is the inclusive boundary");
    let two_transforms = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><g transform='translate(1 1) scale(2)'/></svg>";
    assert_eq!(
        decode_vector_bounded_with_limits(two_transforms, &image, &vector, Some(VectorFormat::Svg)),
        Err(VectorImageError::WorkLimit {
            work: VectorWork::TransformOperations,
            observed: 2,
            limit: 1
        })
    );
    let filter = b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><filter id='f'><feTurbulence/></filter></svg>";
    assert_eq!(
        decode_vector_bounded_with_limits(
            filter,
            &image,
            &VectorLimits::default(),
            Some(VectorFormat::Svg)
        ),
        Err(VectorImageError::UnsupportedFeature { feature: "filter" })
    );

    let geometry = b"<svg xmlns='http://www.w3.org/2000/svg' width='5' height='4'/>";
    let dimension_limit = ImageLimits {
        max_dimension: 4,
        ..image
    };
    assert!(matches!(
        decode_vector_bounded_with_limits(
            geometry,
            &dimension_limit,
            &VectorLimits::default(),
            Some(VectorFormat::Svg)
        ),
        Err(VectorImageError::DimensionTooLarge { width: 5, .. })
    ));
    let allocation_limit = ImageLimits {
        max_pixels: 20,
        max_allocation_bytes: 79,
        ..image
    };
    assert_eq!(
        decode_vector_bounded_with_limits(
            geometry,
            &allocation_limit,
            &VectorLimits::default(),
            Some(VectorFormat::Svg)
        ),
        Err(VectorImageError::AllocationTooLarge {
            bytes: 80,
            limit: 79
        })
    );

    assert!(matches!(
        termleaf::document::image::decode_bounded(b"\x1f\x8bnot gzip"),
        Err(ImageResourceError::Vector(VectorImageError::GzipCorrupt))
    ));
}

fn preflight(path: &str, xml: &[u8], xml_limits: XmlLimits) -> Result<(), ArchiveError> {
    let limits = ArchiveLimits {
        max_compressed_bytes: 64 * 1024,
        max_members: 16,
        max_advertised_expansion: 32 * 1024,
        max_control_member: 16 * 1024,
        max_chapter_member: 16 * 1024,
        max_compression_ratio: 100,
        small_file_exception: 16 * 1024,
        xml: xml_limits,
    };
    PreflightedArchive::open(archive_bytes(&[(path, xml)]), "security.epub", &limits).map(|_| ())
}

#[test]
fn sec_009_depth_and_opening_boundaries_cover_every_control_kind() {
    for path in CONTROL_PATHS {
        let depth_limits = XmlLimits {
            max_depth: 4,
            max_nodes: 16,
        };
        preflight(path, &nested(4), depth_limits).expect("exact depth is inclusive");
        let error = preflight(path, &nested(5), depth_limits).expect_err("depth + 1 rejects");
        assert!(
            matches!(
                error,
                ArchiveError::UnsafeXmlStructure {
                    source: XmlStructureError::TooDeep { depth: 5, limit: 4 },
                    ..
                }
            ),
            "{path}: {error:?}"
        );

        let node_limits = XmlLimits {
            max_depth: 4,
            max_nodes: 4,
        };
        preflight(path, &wide(4), node_limits).expect("exact node count is inclusive");
        let error = preflight(path, &wide(5), node_limits).expect_err("node count + 1 rejects");
        assert!(
            matches!(
                error,
                ArchiveError::UnsafeXmlStructure {
                    source: XmlStructureError::TooManyNodes { nodes: 5, limit: 4 },
                    ..
                }
            ),
            "{path}: {error:?}"
        );
    }

    assert_eq!(
        XmlLimits::default(),
        XmlLimits {
            max_depth: 256,
            max_nodes: 1_000_000,
        }
    );
}

#[test]
fn sec_009_dtd_and_entity_declarations_reject_before_package_semantics() {
    let declarations: [&[u8]; 4] = [
        b"<!DOCTYPE package SYSTEM 'file:///etc/passwd'><package/>",
        b"<!ENTITY secret SYSTEM 'file:///etc/passwd'><package/>",
        b"<!ELEMENT package ANY><package/>",
        b"<!ATTLIST package id ID #IMPLIED><package/>",
    ];
    for path in CONTROL_PATHS {
        for declaration in declarations {
            let error = preflight(path, declaration, XmlLimits::default())
                .expect_err("declarations never reach semantic parsing");
            assert!(
                matches!(
                    error,
                    ArchiveError::UnsafeXmlStructure {
                        source: XmlStructureError::DtdDeclaration,
                        ..
                    }
                ),
                "{path}: {error:?}"
            );
        }
    }

    validate_xml_structure(
        b"<!-- <!DOCTYPE ignored> --><root><![CDATA[<!ENTITY ignored>]]></root>",
        XmlLimits::default(),
    )
    .expect("declaration-like text in inert XML sections is not a declaration");
}

#[test]
fn prop_007_checked_archive_arithmetic_is_monotonic_and_never_wraps() {
    assert_eq!(checked_expansion_total(7, 3, 10), Some(10));
    assert_eq!(checked_expansion_total(7, 4, 10), None);
    assert_eq!(
        checked_expansion_total(u64::MAX - 1, 1, u64::MAX),
        Some(u64::MAX)
    );
    assert_eq!(checked_expansion_total(u64::MAX - 1, 2, u64::MAX), None);

    for member_count in 0..=8 {
        let names: Vec<String> = (0..member_count)
            .map(|index| format!("m{index}.bin"))
            .collect();
        let bodies: Vec<Vec<u8>> = (0..member_count).map(|index| vec![0; index + 1]).collect();
        let members: Vec<(&str, &[u8])> = names
            .iter()
            .zip(&bodies)
            .map(|(name, body)| (name.as_str(), body.as_slice()))
            .collect();
        let bytes = archive_bytes(&members);
        let expansion: u64 = bodies.iter().map(|body| body.len() as u64).sum();

        for member_limit in 0..=9 {
            for expansion_limit in expansion.saturating_sub(1)..=expansion + 1 {
                let limits = ArchiveLimits {
                    max_compressed_bytes: bytes.len() as u64,
                    max_members: member_limit,
                    max_advertised_expansion: expansion_limit,
                    ..ArchiveLimits::default()
                };
                let accepted =
                    PreflightedArchive::open(bytes.clone(), "prop.epub", &limits).is_ok();
                assert_eq!(
                    accepted,
                    member_count <= member_limit && expansion <= expansion_limit,
                    "members={member_count}/{member_limit}, expansion={expansion}/{expansion_limit}"
                );
            }
        }
    }
}

fn tree_entries(root: &Path) -> BTreeSet<PathBuf> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(path).expect("read isolated root") {
            let entry = entry.expect("directory entry");
            let child = entry.path();
            entries.insert(child.strip_prefix(root).expect("inside root").to_owned());
            if child.is_dir() {
                visit(root, &child, entries);
            }
        }
    }

    let mut entries = BTreeSet::new();
    visit(root, root, &mut entries);
    entries
}

#[test]
fn epub_015_successful_read_creates_no_extraction_tree_or_sidecar() {
    const CHILD_BOOK: &str = "TERMLEAF_EPUB_015_CHILD_BOOK";
    if let Some(book_path) = std::env::var_os(CHILD_BOOK) {
        let document = EpubSnapshot::open(Path::new(&book_path), &ArchiveLimits::default())
            .expect("child preflight succeeds")
            .build()
            .expect("child semantic build succeeds");
        assert!(document.canonical().contains("archive only"));
        return;
    }

    let root = tempfile::tempdir().expect("isolated filesystem root");
    let book_path = root.path().join("book.epub");
    let temp_path = root.path().join("tmp");
    let cache_path = root.path().join("cache");
    let config_path = root.path().join("config");
    let home_path = root.path().join("home");
    for directory in [&temp_path, &cache_path, &config_path, &home_path] {
        std::fs::create_dir(directory).expect("create monitored directory");
    }
    let container = br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    let opf = br#"<package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Isolated</dc:title><dc:language>en</dc:language><dc:identifier id="id">id</dc:identifier></metadata><manifest><item id="c" href="c.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c"/></spine></package>"#;
    let chapter =
        br#"<html xmlns="http://www.w3.org/1999/xhtml"><body><p>archive only</p></body></html>"#;
    std::fs::write(
        &book_path,
        archive_bytes(&[
            ("mimetype", b"application/epub+zip"),
            ("META-INF/container.xml", container),
            ("OEBPS/content.opf", opf),
            ("OEBPS/c.xhtml", chapter),
        ]),
    )
    .expect("write isolated EPUB");
    let source_before = std::fs::read(&book_path).expect("snapshot source");
    let entries_before = tree_entries(root.path());

    let status = std::process::Command::new(std::env::current_exe().expect("security test binary"))
        .args([
            "--exact",
            "epub_015_successful_read_creates_no_extraction_tree_or_sidecar",
        ])
        .env(CHILD_BOOK, &book_path)
        .env("TMPDIR", &temp_path)
        .env("TEMP", &temp_path)
        .env("TMP", &temp_path)
        .env("XDG_CACHE_HOME", &cache_path)
        .env("XDG_CONFIG_HOME", &config_path)
        .env("HOME", &home_path)
        .status()
        .expect("run isolated child");
    assert!(status.success(), "isolated EPUB read failed");
    assert_eq!(
        std::fs::read(&book_path).expect("reread source"),
        source_before
    );
    assert_eq!(tree_entries(root.path()), entries_before);
}
