//! Tolerant XHTML-to-block conversion for EPUB chapters.
//!
//! Real books contain XHTML that browsers recover from safely, so chapters
//! parse through the HTML5 tree builder (`scraper`) instead of a strict XML
//! parser. Only meaningful reading structure survives: headings, paragraphs,
//! and line breaks; scripts, styles, and everything the terminal cannot show
//! is dropped on the floor rather than executed or rendered.

use scraper::{Html, Node};

use super::model::BlockKind;

/// One converted logical block with its display text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBlock {
    /// Semantic role for the shared document model.
    pub kind: BlockKind,
    /// Visible text after entity decoding and whitespace collapsing.
    pub text: String,
}

/// Recursion guard: hostile nesting deeper than this contributes nothing.
///
/// Browsers cap element depth similarly; a stack overflow would crash the
/// reader before any policy check could run.
const MAX_XHTML_DEPTH: usize = 128;

/// Converts one chapter's XHTML source into semantic blocks.
///
/// Malformed but recoverable markup still yields its readable text; empty
/// results are dropped so blank chapters contribute no blocks.
#[must_use]
pub fn convert_xhtml(source: &str) -> Vec<SemanticBlock> {
    let document = Html::parse_document(source);
    let root = document.tree.root();
    let mut blocks = Vec::new();
    walk(root, 0, &mut blocks);
    blocks.retain(|block| !block.text.is_empty());
    blocks
}

fn walk(node: ego_tree::NodeRef<'_, Node>, depth: usize, blocks: &mut Vec<SemanticBlock>) {
    if depth > MAX_XHTML_DEPTH {
        return;
    }
    match node.value() {
        Node::Element(element) => {
            let name = element.name();
            if matches!(name, "script" | "style" | "head" | "template") {
                return;
            }
            if let Some(level) = heading_level(name) {
                let text = collapse_inline(&node, depth);
                if !text.is_empty() {
                    blocks.push(SemanticBlock {
                        kind: BlockKind::Heading { level },
                        text,
                    });
                }
                return;
            }
            if name == "p" || name == "li" {
                let text = collapse_inline(&node, depth);
                if !text.is_empty() {
                    blocks.push(SemanticBlock {
                        kind: BlockKind::Paragraph,
                        text,
                    });
                }
                // Paragraph-like containers never recurse; their subtree was
                // already consumed as inline content.
                return;
            }
            for child in node.children() {
                walk(child, depth + 1, blocks);
            }
        }
        _ => {
            for child in node.children() {
                walk(child, depth + 1, blocks);
            }
        }
    }
}

fn heading_level(name: &str) -> Option<u8> {
    let level = match name {
        "h1" => 1,
        "h2" => 2,
        "h3" => 3,
        "h4" => 4,
        "h5" => 5,
        "h6" => 6,
        _ => return None,
    };
    Some(level)
}

/// Collects visible inline text under `node`.
///
/// Line-break elements become logical newlines between collapsed segments;
/// every other descendant contributes decoded, whitespace-collapsed text.
fn collapse_inline(node: &ego_tree::NodeRef<'_, Node>, depth: usize) -> String {
    if depth > MAX_XHTML_DEPTH {
        return String::new();
    }
    let mut segments = vec![String::new()];
    collect_inline(node, depth, &mut segments);
    segments
        .iter()
        .map(|segment| segment.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_inline(node: &ego_tree::NodeRef<'_, Node>, depth: usize, segments: &mut Vec<String>) {
    if depth > MAX_XHTML_DEPTH {
        return;
    }
    match node.value() {
        Node::Text(text) => segments.last_mut().expect("seeded").push_str(text),
        Node::Element(element) => {
            if matches!(element.name(), "script" | "style") {
                return;
            }
            if element.name() == "br" {
                segments.push(String::new());
                return;
            }
            for child in node.children() {
                collect_inline(&child, depth + 1, segments);
            }
        }
        _ => {
            for child in node.children() {
                collect_inline(&child, depth + 1, segments);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(blocks: &[SemanticBlock]) -> Vec<&str> {
        blocks.iter().map(|block| block.text.as_str()).collect()
    }

    #[test]
    fn epub_011_headings_paragraphs_and_breaks_convert_with_entities_decoded() {
        let source = r"<html><head><title>t</title></head><body>
            <h1>Chapter One</h1>
            <p>First &amp; foremost.</p>
            <p>line one<br/>line two</p>
            <h2>Section</h2>
            <p>tail</p>
        </body></html>";
        let blocks = convert_xhtml(source);

        assert_eq!(
            blocks,
            [
                SemanticBlock {
                    kind: BlockKind::Heading { level: 1 },
                    text: "Chapter One".to_owned(),
                },
                SemanticBlock {
                    kind: BlockKind::Paragraph,
                    text: "First & foremost.".to_owned(),
                },
                SemanticBlock {
                    kind: BlockKind::Paragraph,
                    text: "line one\nline two".to_owned(),
                },
                SemanticBlock {
                    kind: BlockKind::Heading { level: 2 },
                    text: "Section".to_owned(),
                },
                SemanticBlock {
                    kind: BlockKind::Paragraph,
                    text: "tail".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn epub_005_malformed_but_recoverable_xhtml_stays_readable() {
        let source = "<body><p>Unclosed paragraph\n   with   odd\tspacing<p>next \
                      <b>bold<i>nested</b> italic</i></p> \
                      <p>&unknownentity &#xZZ; stays literal-safe</p>";
        let blocks = convert_xhtml(source);

        assert_eq!(
            texts(&blocks),
            [
                "Unclosed paragraph with odd spacing",
                "next boldnested italic",
                "&unknownentity &#xZZ; stays literal-safe"
            ]
        );
        assert!(
            blocks
                .iter()
                .all(|block| block.kind == BlockKind::Paragraph)
        );
    }

    #[test]
    fn epub_009_scripts_and_styles_never_contribute_text() {
        let source = "<body><script>alert('x')</script><style>p{color:red}</style>\
                      <p>visible</p></body>";
        assert_eq!(texts(&convert_xhtml(source)), ["visible"]);
    }

    #[test]
    fn epub_005_hostile_nesting_depth_stays_bounded_without_panic() {
        let deep = format!(
            "{}<p>bottom</p>{}",
            "<div>".repeat(4_000),
            "</div>".repeat(4_000)
        );
        let blocks = convert_xhtml(&deep);
        // The innermost paragraph sits far beyond the depth cap and is
        // dropped deterministically instead of overflowing the stack.
        assert!(blocks.is_empty(), "{:?}", texts(&blocks));

        let shallow = format!("{}<p>kept</p>{}", "<div>".repeat(8), "</div>".repeat(8));
        assert_eq!(texts(&convert_xhtml(&shallow)), ["kept"]);
    }
}
