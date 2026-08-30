//! Property-style tests over generated documents, widths, anchors, and
//! action sequences.
//!
//! Generation is deterministic: every case owns a fixed seed and a
//! dependency-free xorshift generator, and failures print the seed plus the
//! failing step so the exact input reproduces. Scheduled runs may raise the
//! iteration counts without changing the generators.

use termleaf::app::{Action, App, StartupOptions};
use termleaf::document::text::{document_from_text, load_text_bytes};
use termleaf::document::{BlockKind, Document, DocumentError, DocumentId, TextLimits};
use termleaf::document::{markdown::load_markdown_bytes, xhtml::convert_xhtml};
use termleaf::layout::layout_document;
use termleaf::reader::{self, Direction};
use unicode_segmentation::UnicodeSegmentation;

/// xorshift64*; fixed seeds keep runs reproducible without a dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            0
        } else {
            let bounded = u64::try_from(bound).unwrap_or(u64::MAX);
            usize::try_from(self.next() % bounded).unwrap_or(0)
        }
    }
}

/// Grapheme-rich building blocks: ASCII words, CJK, a combining sequence, a
/// ZWJ family emoji, a flag, skin tone, wide punctuation, and a tab.
const PIECES: &[&str] = &[
    "word",
    "a",
    "two words",
    "漢字",
    "かな",
    "。",
    "e\u{301}",
    "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    "\u{1F1FA}\u{1F1F8}",
    "\u{1F44D}\u{1F3FD}",
    "tab\tstop",
];

fn generated_document(rng: &mut Rng, seed: u64) -> Document {
    let mut source = String::new();
    let paragraphs = 1 + rng.below(5);
    for paragraph in 0..paragraphs {
        let lines = 1 + rng.below(4);
        for line in 0..lines {
            let pieces = rng.below(8);
            for _ in 0..pieces {
                source.push_str(PIECES[rng.below(PIECES.len())]);
                if rng.below(4) == 0 {
                    source.push(' ');
                }
            }
            if line + 1 < lines || paragraph + 1 < paragraphs {
                source.push('\n');
            }
        }
        if paragraph + 1 < paragraphs {
            source.push_str("\n\n");
        }
    }
    document_from_text(DocumentId::new(format!("prop-{seed}")), None, &source)
        .unwrap_or_else(|error| panic!("seed {seed:#x}: generated text must parse: {error}"))
}

fn assert_valid_text_result(
    result: Result<Document, DocumentError>,
    source_len: usize,
    limit: u64,
    context: &str,
) {
    match result {
        Ok(document) => {
            assert!(
                u64::try_from(source_len).is_ok_and(|len| len <= limit),
                "{context}: an over-limit source succeeded"
            );
            assert_eq!(document.sections().len(), 1, "{context}");
            let mut covered = 0usize;
            for block in document.sections()[0].blocks() {
                assert_eq!(block.range().start, covered, "{context}");
                assert!(block.range().end <= document.len(), "{context}");
                assert!(
                    document.canonical().is_char_boundary(block.range().start)
                        && document.canonical().is_char_boundary(block.range().end),
                    "{context}"
                );
                covered = block.range().end;
            }
            assert_eq!(covered, document.len(), "{context}");
            assert!(
                document.len() <= source_len.saturating_mul(3),
                "{context}: decoded model escaped its source-derived bound"
            );
        }
        Err(DocumentError::TooLarge {
            size,
            limit: actual,
            ..
        }) => {
            assert_eq!(actual, limit, "{context}");
            assert_eq!(
                size,
                u64::try_from(source_len).unwrap_or(u64::MAX),
                "{context}"
            );
            assert!(size > limit, "{context}");
        }
        Err(DocumentError::InvalidEncoding { offset, .. }) => {
            assert!(
                u64::try_from(source_len).is_ok_and(|len| len <= limit),
                "{context}: encoding validation preceded the size bound"
            );
            assert!(offset <= source_len, "{context}: invalid offset {offset}");
        }
        Err(DocumentError::InvalidUtf16 { detail, .. }) => {
            assert!(
                u64::try_from(source_len).is_ok_and(|len| len <= limit),
                "{context}: UTF-16 validation preceded the size bound"
            );
            assert!(detail.contains("byte offset"), "{context}: {detail}");
        }
        Err(other) => panic!("{context}: unexpected text source error: {other:?}"),
    }
}

fn sample_widths(rng: &mut Rng) -> Vec<u16> {
    let mut widths: Vec<u16> = [
        1_u16, 2, 3, 7, 8, 15, 16, 39, 40, 79, 80, 119, 120, 199, 200,
    ]
    .into_iter()
    .collect();
    for _ in 0..6 {
        widths.push(u16::try_from(1 + rng.below(200)).unwrap_or(1));
    }
    widths
}

#[test]
fn prop_001_no_row_exceeds_width_and_layout_terminates() {
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..200 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);
        let canonical_len = document.len();

        for width in sample_widths(&mut rng) {
            let layout = layout_document(&document, width);
            assert!(
                layout.rows().len() <= canonical_len + document.sections()[0].blocks().len(),
                "seed {seed:#x} width {width}: rows {} exceed a bounded count",
                layout.rows().len()
            );
            for row in layout.rows() {
                let fits = row.cells() <= width;
                let raw_graphemes: usize = row
                    .spans()
                    .iter()
                    .map(|span| {
                        document
                            .canonical()
                            .get(span.range().clone())
                            .map_or(0, |raw| raw.graphemes(true).count())
                    })
                    .sum();
                let single_cluster_overflow = raw_graphemes == 1;
                assert!(
                    fits || single_cluster_overflow,
                    "seed {seed:#x} width {width}: row used {} cells and carries {:?}",
                    row.cells(),
                    row.text(&document)
                );
            }
        }
    }
}

#[test]
fn prop_002_spans_never_split_graphemes() {
    use unicode_segmentation::UnicodeSegmentation;

    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..200 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);

        let section = &document.sections()[0];
        for (index, block) in section.blocks().iter().enumerate() {
            if block.kind() != BlockKind::Paragraph {
                continue;
            }
            let Some(text) = document.block_text(0, index) else {
                continue;
            };
            let boundaries: std::collections::HashSet<usize> = text
                .grapheme_indices(true)
                .map(|(offset, _)| offset)
                .chain(std::iter::once(text.len()))
                .collect();

            for width in sample_widths(&mut rng) {
                let layout = layout_document(&document, width);
                for row in layout.rows().iter().filter(|row| row.block() == index) {
                    for span in row.spans() {
                        let relative = span.range().end - block.range().start;
                        assert!(
                            boundaries.contains(&relative),
                            "seed {seed:#x} width {width}: span ends inside a grapheme \
                             at block-relative byte {relative}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn prop_003_anchor_passage_survives_resize_sequences() {
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..200 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);
        if document.is_empty() {
            continue;
        }
        // Anchor at a character boundary near the middle of the book.
        let anchor_byte = document.len() / 2;
        let anchor_byte = {
            let mut candidate = anchor_byte;
            while !document.canonical().is_char_boundary(candidate) {
                candidate -= 1;
            }
            candidate
        };

        for width in sample_widths(&mut rng) {
            let layout = layout_document(&document, width);
            let index = layout.row_after(anchor_byte);
            let covered = layout.rows()[..=index]
                .iter()
                .flat_map(termleaf::layout::VisualRow::spans)
                .map(|span| span.range().end)
                .max()
                .unwrap_or_default();
            assert!(
                covered > anchor_byte || index + 1 == layout.rows().len(),
                "seed {seed:#x} width {width}: anchor byte {anchor_byte} lost its passage"
            );
        }
    }
}

#[test]
fn prop_004_next_page_progresses_then_clamps_at_the_end() {
    let mut rng = Rng::new(0x5EED_0004);
    for _ in 0..200 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);
        let layout = layout_document(&document, 24);
        let Some(start) = document.first_position().ok() else {
            continue;
        };

        let mut anchor = start;
        let mut steps = 0_usize;
        while let Some(next) =
            reader::step_page(&layout, &document, anchor, 7, Direction::TowardEnd)
        {
            assert!(
                next.absolute_byte(&document) >= anchor.absolute_byte(&document),
                "seed {seed:#x}: next page moved backwards"
            );
            if next == anchor {
                break;
            }
            anchor = next;
            steps += 1;
            assert!(steps <= layout.rows().len() + 2, "seed {seed:#x}: no loop");
        }

        // From any interior anchor, next then previous page round-trips
        // exactly whenever the forward hop fit unclamped and crossed no
        // blank spacer rows; end clamping is lossy by definition.
        let Some(mid) = reader::step_line(&layout, &document, start, Direction::TowardEnd) else {
            continue;
        };
        let Some(down) = reader::step_page(&layout, &document, mid, 3, Direction::TowardEnd) else {
            continue;
        };
        let last_index = layout.rows().len() - 1;
        let mid_row = layout.row_after(mid.absolute_byte(&document));
        let down_row = layout.row_after(down.absolute_byte(&document));
        let blanks_between = layout.rows()[mid_row..=down_row]
            .iter()
            .filter(|row| row.spans().is_empty())
            .count();
        let up = reader::step_page(&layout, &document, down, 3, Direction::TowardStart);

        if blanks_between == 0 && mid_row + 3 <= last_index {
            assert_eq!(
                up,
                Some(mid),
                "seed {seed:#x}: an unclamped blank-free page pair must return to its origin"
            );
        } else {
            // The defined policy allows a shorter hop across blank joiners
            // but never a forward drift past the page it started from.
            let up_byte = up.expect("backward step exists").absolute_byte(&document);
            assert!(
                up_byte <= down.absolute_byte(&document),
                "seed {seed:#x}: previous page drifted forward"
            );
        }
    }
}

#[test]
fn nav_010_resize_between_navigation_actions_keeps_movement_valid() {
    let mut rng = Rng::new(0x5EED_000A);
    for _ in 0..150 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);
        let Some(start) = document.first_position().ok() else {
            continue;
        };
        let mut anchor = start;

        for width in sample_widths(&mut rng) {
            let layout = layout_document(&document, width);
            for direction in [Direction::TowardEnd, Direction::TowardStart] {
                if let Some(next) = reader::step_line(&layout, &document, anchor, direction) {
                    let byte = next.absolute_byte(&document);
                    assert!(
                        document
                            .canonical()
                            .is_char_boundary(byte.min(document.len())),
                        "seed {seed:#x} width {width}: line step left an invalid anchor"
                    );
                    anchor = next;
                }
                if let Some(next) = reader::step_page(&layout, &document, anchor, 5, direction) {
                    anchor = next;
                }
                assert!(
                    anchor.absolute_byte(&document) <= document.len(),
                    "seed {seed:#x} width {width}: anchor escaped the document"
                );
            }
        }
    }
}

#[test]
fn prop_010_action_sequences_keep_state_valid_or_typed() {
    const ACTIONS: [Action; 14] = [
        Action::NextLine,
        Action::PreviousLine,
        Action::NextPage,
        Action::PreviousPage,
        Action::DocumentStart,
        Action::DocumentEnd,
        Action::SectionStart,
        Action::SectionEnd,
        Action::SetModePaged,
        Action::SetModeContinuous,
        Action::ShowHelp,
        Action::Back,
        Action::ShowThemes,
        Action::Confirm,
    ];

    let mut rng = Rng::new(0x5EED_000B);
    for _ in 0..60 {
        let seed = rng.next();
        let document = generated_document(&mut rng, seed);
        let file = tempfile::Builder::new()
            .prefix("prop-state")
            .suffix(".txt")
            .tempfile()
            .expect("state fixture");
        std::fs::write(file.path(), document.canonical()).expect("write state fixture");

        let mut app = App::open(StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..StartupOptions::default()
        })
        .expect("generated book opens");
        app.set_content_viewport(60, 15);

        for step in 0..120 {
            let action = ACTIONS[rng.below(ACTIONS.len())];
            app.update(action);

            // Overlay stacking: whenever an overlay view is active, repeated
            // Back presses must change the view each time and eventually
            // land on the reader or the home screen, never panicking or
            // stranding the session.
            let overlay_active = matches!(
                app.view(),
                termleaf::app::View::Help { .. } | termleaf::app::View::ThemeSelection { .. }
            );
            if overlay_active && action == Action::Back {
                let before = app.view().clone();
                let mut presses = 0;
                while matches!(
                    app.view(),
                    termleaf::app::View::Help { .. } | termleaf::app::View::ThemeSelection { .. }
                ) {
                    app.update(Action::Back);
                    presses += 1;
                    // One Back unwinds one stacked overlay; the stack is
                    // bounded by the opening actions seen so far.
                    assert!(
                        presses <= 122,
                        "seed {seed:#x}: return stack did not unwind"
                    );
                }
                assert_ne!(
                    app.view(),
                    &before,
                    "seed {seed:#x} step {step}: Back must leave the overlay"
                );
                assert!(
                    matches!(
                        app.view(),
                        termleaf::app::View::Reader { .. } | termleaf::app::View::RecentBooks
                    ),
                    "seed {seed:#x}: overlays return to a reading surface, got {:?}",
                    app.view()
                );
            }

            if let Some(session) = app.reader() {
                let byte = session.anchor().absolute_byte(session.document());
                assert!(
                    session
                        .document()
                        .canonical()
                        .is_char_boundary(byte.min(document.len())),
                    "seed {seed:#x} step {step}: anchor byte {byte} splits a character"
                );
            }
            assert!(app.is_running(), "seed {seed:#x}: only Quit stops the loop");
            assert!(
                termleaf::ui::theme::ThemeName::ALL.contains(&app.theme()),
                "seed {seed:#x}: theme left the built-in set"
            );
        }

        // Overlays must be unwound before quitting: `q` inside the theme
        // view returns to the reading surface, so the loop exits only from
        // a base view. One Back unwinds one stacked overlay, and the stack
        // is bounded by the 120 opening actions the sequence can contain.
        let mut presses = 0;
        while matches!(
            app.view(),
            termleaf::app::View::Help { .. } | termleaf::app::View::ThemeSelection { .. }
        ) {
            app.update(Action::Back);
            presses += 1;
            assert!(
                presses <= 122,
                "seed {seed:#x}: return stack did not unwind"
            );
        }

        app.update(Action::Quit);
        assert!(!app.is_running());
    }
}

#[test]
fn prop_011_fixed_seed_raw_bytes_are_bounded_models_or_typed_source_errors() {
    let mut rng = Rng::new(0x5EED_0011);
    for case in 0..1_000 {
        let source_len = rng.below(129);
        let mut bytes = Vec::with_capacity(source_len);
        bytes.extend((0..source_len).map(|_| rng.next().to_le_bytes()[0]));
        let limit = u64::try_from(rng.below(129)).unwrap_or(0);
        let context = format!("raw-byte case {case}, length {source_len}, limit {limit}");
        assert_valid_text_result(
            load_text_bytes("raw-property.txt", &bytes, &TextLimits { max_bytes: limit }),
            source_len,
            limit,
            &context,
        );
    }
}

#[test]
fn prop_012_fixed_text_mutations_are_bounded_models_or_typed_source_errors() {
    let sources = [
        b"plain UTF-8\nsecond line\n".to_vec(),
        [0xEF, 0xBB, 0xBF]
            .into_iter()
            .chain(*b"marked UTF-8\n")
            .collect(),
        "UTF-16 LE \u{1F642}\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
        "UTF-16 BE \u{1F642}\n"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>(),
    ];
    let mut sources = sources.to_vec();
    sources[2].splice(0..0, [0xFF, 0xFE]);
    sources[3].splice(0..0, [0xFE, 0xFF]);

    let masks = [0x01, 0x80, 0xFF];
    for (source_index, source) in sources.iter().enumerate() {
        let positions = source.len().min(32);
        for position in 0..positions {
            for mask in masks {
                let mut mutated = source.clone();
                mutated[position] ^= mask;
                let context = format!("source {source_index}, xor {mask:#04x} at {position}");
                let limit = u64::try_from(mutated.len()).unwrap_or(u64::MAX);
                assert_valid_text_result(
                    load_text_bytes(
                        "mutation-property.txt",
                        &mutated,
                        &TextLimits { max_bytes: limit },
                    ),
                    mutated.len(),
                    limit,
                    &context,
                );
            }
        }

        for end in 0..=source.len().min(32) {
            let mutated = &source[..end];
            let context = format!("source {source_index}, truncated at {end}");
            let limit = u64::try_from(mutated.len()).unwrap_or(u64::MAX);
            assert_valid_text_result(
                load_text_bytes(
                    "mutation-property.txt",
                    mutated,
                    &TextLimits { max_bytes: limit },
                ),
                mutated.len(),
                limit,
                &context,
            );
        }

        let above = u64::try_from(source.len().saturating_sub(1)).unwrap_or(u64::MAX);
        assert_valid_text_result(
            load_text_bytes(
                "mutation-property.txt",
                source,
                &TextLimits { max_bytes: above },
            ),
            source.len(),
            above,
            &format!("source {source_index}, exact limit plus one"),
        );
    }
}

#[test]
fn md_009_fixed_seed_markdown_sources_always_form_valid_bounded_models() {
    const TOKENS: &[&str] = &[
        "word",
        " ",
        "\n",
        "#",
        "*",
        "_",
        "`",
        "[",
        "]",
        "(",
        ")",
        "|",
        "<",
        ">",
        "λ",
        "&amp;",
        "<script>x</script>",
    ];
    let mut rng = Rng::new(0x5EED_0013);
    for case in 0..500 {
        let mut source = String::new();
        for _ in 0..rng.below(96) {
            source.push_str(TOKENS[rng.below(TOKENS.len())]);
        }
        let document = load_markdown_bytes(
            "generated.md",
            source.as_bytes(),
            &TextLimits {
                max_bytes: source.len() as u64,
            },
        )
        .unwrap_or_else(|error| panic!("case {case}: bounded UTF-8 must stay typed: {error}"));
        let mut covered = 0usize;
        for block in document.sections()[0].blocks() {
            assert_eq!(block.range().start, covered, "case {case}");
            assert!(block.range().end <= document.len(), "case {case}");
            assert!(document.canonical().is_char_boundary(block.range().end));
            covered = block.range().end;
        }
        assert_eq!(covered, document.len(), "case {case}");
        for span in document.inline_spans() {
            assert!(span.range().end <= document.len(), "case {case}");
            assert!(document.canonical().is_char_boundary(span.range().start));
            assert!(document.canonical().is_char_boundary(span.range().end));
        }
        for mapping in document.source_mappings() {
            assert!(
                mapping.canonical_range().end <= document.len(),
                "case {case}"
            );
            assert!(mapping.source_range().end <= source.len(), "case {case}");
        }
    }
}

#[test]
fn epub_005_fixed_seed_xhtml_models_preserve_all_local_ranges() {
    const TOKENS: &[&str] = &[
        "text",
        " ",
        "<p>",
        "</p>",
        "<em>",
        "</em>",
        "<table>",
        "</table>",
        "<tr><td>",
        "</td></tr>",
        "<img src='x' alt='λ'/>",
        "&amp;",
        "λ",
        "<script>hidden</script>",
    ];
    let mut rng = Rng::new(0x5EED_0014);
    for case in 0..500 {
        let mut source = String::new();
        for _ in 0..rng.below(64) {
            source.push_str(TOKENS[rng.below(TOKENS.len())]);
        }
        let blocks = convert_xhtml(&source)
            .unwrap_or_else(|error| panic!("case {case}: small source rejected: {error}"));
        for block in blocks {
            for range in block
                .inline
                .iter()
                .map(|inline| &inline.range)
                .chain(block.cells.iter())
            {
                assert!(range.start <= range.end && range.end <= block.text.len());
                assert!(block.text.is_char_boundary(range.start));
                assert!(block.text.is_char_boundary(range.end));
            }
            for (_, offset) in block.anchors {
                assert!(offset <= block.text.len());
                assert!(block.text.is_char_boundary(offset));
            }
        }
    }
}
