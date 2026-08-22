//! Tolerant XHTML-to-block conversion for EPUB chapters.
//!
//! Real books contain XHTML that browsers recover from safely, so chapters
//! parse through the HTML5 tree builder (`scraper`) instead of a strict XML
//! parser. Only meaningful reading structure survives: headings, paragraphs,
//! lists, quotes, code, separators, tables, and decorated inline runs;
//! scripts, styles, and everything the terminal cannot show is dropped on
//! the floor rather than executed or rendered.

use std::ops::Range;

use scraper::{Html, Node};

use super::model::{BlockKind, InlineKind};

/// One converted logical block with its display text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticBlock {
    /// Semantic role for the shared document model.
    pub kind: BlockKind,
    /// Visible text after entity decoding and whitespace collapsing.
    pub text: String,
    /// Inline decorations as byte ranges within `text`.
    pub inline: Vec<SemanticInline>,
    /// Row-major cell ranges within `text`; non-empty only for tables.
    pub cells: Vec<Range<usize>>,
}

impl SemanticBlock {
    fn plain(kind: BlockKind, text: String) -> Self {
        Self {
            kind,
            text,
            inline: Vec::new(),
            cells: Vec::new(),
        }
    }
}

/// One inline semantic role over a block-local byte range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticInline {
    /// Byte range within the owning block's text.
    pub range: Range<usize>,
    /// The role the terminal may render with attributes or color.
    pub kind: InlineKind,
}

/// Structural rejection when a chapter exceeds the markup safety budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum XhtmlBoundsError {
    /// The source carries more markup openings than the node budget allows.
    ///
    /// The count runs over raw bytes before any tree allocation, so a hostile
    /// or corrupt chapter stops here instead of inside the HTML5 parser.
    #[error(
        "chapter declares about {nodes} markup nodes beyond the {limit} node limit; \
         the chapter may be corrupt or hostile"
    )]
    TooManyNodes {
        /// Counted markup openings in the rejected source.
        nodes: usize,
        /// Inclusive policy limit that was exceeded.
        limit: usize,
    },
}

/// Recursion guard: hostile nesting deeper than this contributes nothing.
///
/// Browsers cap element depth similarly; a stack overflow would crash the
/// reader before any policy check could run.
const MAX_XHTML_DEPTH: usize = 128;

/// Markup-node safety budget from the EPUB limits table (inclusive).
pub const MAX_XHTML_NODES: usize = 1_000_000;

/// Converts one chapter's XHTML source into semantic blocks.
///
/// Malformed but recoverable markup still yields its readable text; empty
/// results are dropped so blank chapters contribute no blocks. Structure is
/// bounded twice: a byte scan rejects sources above the node budget before
/// the tree builder allocates, and the walk itself caps recursion depth.
///
/// # Errors
///
/// Returns [`XhtmlBoundsError::TooManyNodes`] when the source declares more
/// markup than [`MAX_XHTML_NODES`] allows; parsing never starts in that case.
pub fn convert_xhtml(source: &str) -> Result<Vec<SemanticBlock>, XhtmlBoundsError> {
    convert_xhtml_with_limits(source, MAX_XHTML_NODES)
}

/// Converts one chapter with an explicit node budget for boundary testing.
///
/// # Errors
///
/// Returns [`XhtmlBoundsError::TooManyNodes`] exactly at `max_nodes + 1`
/// markup openings; `max_nodes` and below proceed to conversion.
pub fn convert_xhtml_with_limits(
    source: &str,
    max_nodes: usize,
) -> Result<Vec<SemanticBlock>, XhtmlBoundsError> {
    // Every element, comment, and processing instruction consumes at least
    // one '<', while plain text never does. The count therefore bounds the
    // tree the builder can allocate without parsing anything.
    let openings = source.bytes().filter(|byte| *byte == b'<').count();
    if openings > max_nodes {
        return Err(XhtmlBoundsError::TooManyNodes {
            nodes: openings,
            limit: max_nodes,
        });
    }

    let document = Html::parse_document(source);
    let root = document.tree.root();
    let mut blocks = Vec::new();
    walk(root, 0, &mut blocks);
    blocks.retain(|block| !block.text.is_empty());
    Ok(blocks)
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
                push_decorated(
                    blocks,
                    BlockKind::Heading { level },
                    collapse_inline(&node, depth),
                );
                // Heading containers never recurse; their subtree was
                // already consumed as decorated inline content.
                return;
            }
            if name == "p" {
                push_decorated(blocks, BlockKind::Paragraph, collapse_inline(&node, depth));
                return;
            }
            if name == "ul" || name == "ol" {
                let ordered = name == "ol";
                for child in node.children() {
                    match child.value() {
                        Node::Element(child_element) if child_element.name() == "li" => {
                            push_list_item(&child, depth, 0, ordered, blocks);
                        }
                        _ => walk(child, depth + 1, blocks),
                    }
                }
                return;
            }
            if name == "blockquote" {
                push_decorated(blocks, BlockKind::Quote, collapse_inline(&node, depth));
                return;
            }
            if name == "pre" {
                let code = literal_text(&node, depth);
                if !code.is_empty() {
                    blocks.push(SemanticBlock::plain(BlockKind::CodeBlock, code));
                }
                return;
            }
            if name == "hr" {
                blocks.push(SemanticBlock::plain(
                    BlockKind::Separator,
                    "* * *".to_owned(),
                ));
                return;
            }
            if name == "table" {
                if let Some(table) = convert_table(&node, depth) {
                    blocks.push(table);
                }
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

/// Pushes one block built from a decorated collapse when it has content.
fn push_decorated(
    blocks: &mut Vec<SemanticBlock>,
    kind: BlockKind,
    collapsed: (String, Vec<SemanticInline>),
) {
    let (text, inline) = collapsed;
    if !text.is_empty() {
        blocks.push(SemanticBlock {
            kind,
            text,
            inline,
            cells: Vec::new(),
        });
    }
}

/// Emits one list item plus any nested lists as deeper sibling items.
fn push_list_item(
    node: &ego_tree::NodeRef<'_, Node>,
    depth: usize,
    list_depth: u8,
    ordered: bool,
    blocks: &mut Vec<SemanticBlock>,
) {
    if depth > MAX_XHTML_DEPTH {
        return;
    }
    // The item's own text skips nested lists so they become their own
    // deeper items instead of duplicated inline text.
    let mut builder = InlineBuilder::default();
    builder.break_line();
    collect_inline(node, depth + 1, None, true, &mut builder);
    let (text, kinds) = builder.finish();
    push_decorated(
        blocks,
        BlockKind::ListItem {
            depth: list_depth,
            ordered,
        },
        (text, runs_from_kinds(&kinds)),
    );

    for (list_node, nested_ordered) in nested_lists(*node, depth + 1) {
        let nested_depth = list_depth.saturating_add(1);
        for child in list_node.children() {
            match child.value() {
                Node::Element(child_element) if child_element.name() == "li" => {
                    push_list_item(&child, depth + 1, nested_depth, nested_ordered, blocks);
                }
                _ => walk(child, depth + 1, blocks),
            }
        }
    }
}

/// Finds the first level of `ul`/`ol` elements under `node`.
///
/// Found lists are not descended into; their items recurse through
/// [`push_list_item`], which discovers deeper levels itself.
fn nested_lists(
    node: ego_tree::NodeRef<'_, Node>,
    depth: usize,
) -> Vec<(ego_tree::NodeRef<'_, Node>, bool)> {
    let mut found = Vec::new();
    if depth > MAX_XHTML_DEPTH {
        return found;
    }
    match node.value() {
        Node::Element(element) => {
            let name = element.name();
            if name == "ul" || name == "ol" {
                found.push((node, name == "ol"));
                return found;
            }
            if matches!(name, "script" | "style" | "template") || heading_level(name).is_some() {
                return found;
            }
            for child in node.children() {
                found.extend(nested_lists(child, depth + 1));
            }
        }
        _ => {
            for child in node.children() {
                found.extend(nested_lists(child, depth + 1));
            }
        }
    }
    found
}

/// Builds one table block with row-major cell ranges.
fn convert_table(node: &ego_tree::NodeRef<'_, Node>, depth: usize) -> Option<SemanticBlock> {
    if depth > MAX_XHTML_DEPTH {
        return None;
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for tr in descendants_named(*node, "tr", depth) {
        let mut cells = Vec::new();
        for td in descendants_named(tr, "td", depth + 1)
            .into_iter()
            .chain(descendants_named(tr, "th", depth + 1))
        {
            cells.push(collapse_inline(&td, depth + 1).0);
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }

    let mut text = String::new();
    let mut ranges = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            text.push('\n');
        }
        for (cell_index, cell) in row.iter().enumerate() {
            if cell_index > 0 {
                text.push_str(" | ");
            }
            let start = text.len();
            text.push_str(cell);
            ranges.push(start..text.len());
        }
    }

    Some(SemanticBlock {
        kind: BlockKind::Table,
        text,
        inline: Vec::new(),
        cells: ranges,
    })
}

/// Collects descendant elements with the given name, bounded by depth.
fn descendants_named<'a>(
    node: ego_tree::NodeRef<'a, Node>,
    name: &str,
    depth: usize,
) -> Vec<ego_tree::NodeRef<'a, Node>> {
    let mut found = Vec::new();
    if depth > MAX_XHTML_DEPTH {
        return found;
    }
    match node.value() {
        Node::Element(element) => {
            if matches!(element.name(), "script" | "style" | "template") {
                return found;
            }
            if element.name() == name {
                found.push(node);
                return found;
            }
            for child in node.children() {
                found.extend(descendants_named(child, name, depth + 1));
            }
        }
        _ => {
            for child in node.children() {
                found.extend(descendants_named(child, name, depth + 1));
            }
        }
    }
    found
}

/// Collects literal source text under `pre`, preserving whitespace.
///
/// Scripts and styles never contribute; every other descendant's decoded
/// text joins without collapsing so indentation and newlines survive.
fn literal_text(node: &ego_tree::NodeRef<'_, Node>, depth: usize) -> String {
    let mut out = String::new();
    if depth > MAX_XHTML_DEPTH {
        return out;
    }
    match node.value() {
        Node::Text(text) => out.push_str(text),
        Node::Element(element) => {
            if matches!(element.name(), "script" | "style" | "template") {
                return out;
            }
            for child in node.children() {
                out.push_str(&literal_text(&child, depth + 1));
            }
        }
        _ => {
            for child in node.children() {
                out.push_str(&literal_text(&child, depth + 1));
            }
        }
    }
    out
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

/// Collects visible inline text under `node` with its semantic roles.
///
/// Line-break elements become logical newlines between collapsed segments;
/// every other descendant contributes decoded, whitespace-collapsed text.
/// The returned kinds vector holds one entry per output byte so decoration
/// ranges survive collapsing exactly.
fn collapse_inline(
    node: &ego_tree::NodeRef<'_, Node>,
    depth: usize,
) -> (String, Vec<SemanticInline>) {
    let mut builder = InlineBuilder::default();
    builder.break_line();
    collect_inline(node, depth, None, false, &mut builder);
    let (text, kinds) = builder.finish();
    (text, runs_from_kinds(&kinds))
}

/// One entry per byte of collapsed text: the active inline role.
type ByteKinds = Vec<Option<InlineKind>>;

/// Accumulates collapsed segments plus their per-byte roles.
#[derive(Default)]
struct InlineBuilder {
    segments: Vec<String>,
    kinds: Vec<ByteKinds>,
    pending_space: bool,
    /// Role active when the pending whitespace appeared, so a collapsed
    /// separator inherits its own enclosing context rather than the next
    /// element's.
    pending_kind: Option<InlineKind>,
}

impl InlineBuilder {
    fn break_line(&mut self) {
        self.segments.push(String::new());
        self.kinds.push(Vec::new());
        self.pending_space = false;
        self.pending_kind = None;
    }

    fn push_char(&mut self, character: char, kind: Option<InlineKind>) {
        if character.is_whitespace() {
            self.pending_space = true;
            self.pending_kind = kind;
            return;
        }
        let segment = self.segments.last_mut().expect("seeded");
        if self.pending_space && !segment.is_empty() {
            segment.push(' ');
            self.kinds
                .last_mut()
                .expect("seeded")
                .push(self.pending_kind);
        }
        self.pending_space = false;
        let mut buffer = [0u8; 4];
        let encoded = character.encode_utf8(&mut buffer);
        segment.push_str(encoded);
        for _ in 0..encoded.len() {
            self.kinds.last_mut().expect("seeded").push(kind);
        }
    }

    /// Joins non-empty segments with newlines and returns byte-level roles.
    fn finish(self) -> (String, ByteKinds) {
        let mut text = String::new();
        let mut kinds = ByteKinds::new();
        for (segment, mut segment_kinds) in self.segments.into_iter().zip(self.kinds) {
            if segment.is_empty() {
                continue;
            }
            if !text.is_empty() {
                text.push('\n');
                kinds.push(None);
            }
            text.push_str(&segment);
            kinds.append(&mut segment_kinds);
        }
        (text, kinds)
    }
}

/// Groups equal adjacent byte roles into ordered decoration ranges.
fn runs_from_kinds(kinds: &[Option<InlineKind>]) -> Vec<SemanticInline> {
    let mut runs = Vec::new();
    let mut index = 0usize;
    while index < kinds.len() {
        let kind = kinds[index];
        let mut end = index + 1;
        while end < kinds.len() && kinds[end] == kind {
            end += 1;
        }
        if let Some(decoration) = kind {
            runs.push(SemanticInline {
                range: index..end,
                kind: decoration,
            });
        }
        index = end;
    }
    runs
}

fn collect_inline(
    node: &ego_tree::NodeRef<'_, Node>,
    depth: usize,
    kind: Option<InlineKind>,
    skip_lists: bool,
    builder: &mut InlineBuilder,
) {
    if depth > MAX_XHTML_DEPTH {
        return;
    }
    match node.value() {
        Node::Text(text) => {
            for character in text.chars() {
                builder.push_char(character, kind);
            }
        }
        Node::Element(element) => {
            let name = element.name();
            if matches!(name, "script" | "style" | "template") {
                return;
            }
            if name == "br" {
                builder.break_line();
                return;
            }
            if skip_lists && matches!(name, "ul" | "ol" | "table") {
                return;
            }
            let child_kind = inline_kind_for(name).or(kind);
            for child in node.children() {
                collect_inline(&child, depth + 1, child_kind, skip_lists, builder);
            }
        }
        _ => {
            for child in node.children() {
                collect_inline(&child, depth + 1, kind, skip_lists, builder);
            }
        }
    }
}

fn inline_kind_for(name: &str) -> Option<InlineKind> {
    match name {
        "em" | "i" | "cite" | "dfn" | "var" => Some(InlineKind::Emphasis),
        "strong" | "b" => Some(InlineKind::Strong),
        "code" | "kbd" | "samp" | "tt" => Some(InlineKind::Code),
        "a" => Some(InlineKind::Link),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(blocks: &[SemanticBlock]) -> Vec<&str> {
        blocks.iter().map(|block| block.text.as_str()).collect()
    }

    fn plain(kind: BlockKind, text: &str) -> SemanticBlock {
        SemanticBlock::plain(kind, text.to_owned())
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
        let blocks = convert_xhtml(source).expect("within node budget");

        assert_eq!(
            blocks,
            [
                plain(BlockKind::Heading { level: 1 }, "Chapter One"),
                plain(BlockKind::Paragraph, "First & foremost."),
                plain(BlockKind::Paragraph, "line one\nline two"),
                plain(BlockKind::Heading { level: 2 }, "Section"),
                plain(BlockKind::Paragraph, "tail"),
            ]
        );
    }

    #[test]
    fn epub_011_inline_roles_map_emphasis_strong_code_and_links() {
        let source = r#"<body>
            <p>plain <em>tilted</em> and <strong>heavy</strong> plus
               <code>let x = 1;</code> and a <a href="chapter.xhtml#note">link text</a>.</p>
        </body>"#;
        let blocks = convert_xhtml(source).expect("within node budget");
        assert_eq!(blocks.len(), 1);
        let block = &blocks[0];
        assert_eq!(
            block.text,
            "plain tilted and heavy plus let x = 1; and a link text."
        );

        let expected = [
            ("plain ", None),
            ("tilted", Some(InlineKind::Emphasis)),
            (" and ", None),
            ("heavy", Some(InlineKind::Strong)),
            (" plus ", None),
            ("let x = 1;", Some(InlineKind::Code)),
            (" and a ", None),
            ("link text", Some(InlineKind::Link)),
            (".", None),
        ];
        for (fragment, kind) in expected {
            let position = block.text.find(fragment).expect("fragment present");
            let covered = block.inline.iter().any(|run| {
                run.range.start >= position
                    && run.range.start + fragment.len() <= position + fragment.len()
                    && run.kind == kind.unwrap_or(InlineKind::Emphasis)
                    && kind.is_some()
            });
            if kind.is_some() {
                assert!(covered, "'{fragment}' carries {kind:?}");
            } else {
                let undecorated = block.inline.iter().all(|run| {
                    run.range.end <= position || run.range.start >= position + fragment.len()
                });
                assert!(undecorated, "'{fragment}' stays plain");
            }
        }
    }

    #[test]
    fn epub_011_lists_quotes_code_separators_convert_with_structure() {
        let source = r"<body>
            <h1>Garden</h1>
            <ul><li>roses</li><li>ferns<ol><li>first</li></ol></li></ul>
            <blockquote>spoken softly here</blockquote>
            <pre>fn keep() {
    return 1;
}</pre>
            <hr/>
            <p>after the rule</p>
        </body>";
        let blocks = convert_xhtml(source).expect("within node budget");

        let kinds: Vec<BlockKind> = blocks.iter().map(|block| block.kind).collect();
        assert_eq!(
            kinds,
            [
                BlockKind::Heading { level: 1 },
                BlockKind::ListItem {
                    depth: 0,
                    ordered: false
                },
                BlockKind::ListItem {
                    depth: 0,
                    ordered: false
                },
                BlockKind::ListItem {
                    depth: 1,
                    ordered: true
                },
                BlockKind::Quote,
                BlockKind::CodeBlock,
                BlockKind::Separator,
                BlockKind::Paragraph,
            ]
        );
        assert_eq!(blocks[1].text, "roses");
        assert_eq!(
            blocks[2].text, "ferns",
            "nested list text stays out of item"
        );
        assert_eq!(blocks[3].text, "first");
        assert_eq!(blocks[4].text, "spoken softly here");
        assert_eq!(
            blocks[5].text, "fn keep() {\n    return 1;\n}",
            "code preserves indentation and newlines"
        );
        assert_eq!(blocks[6].text, "* * *");
    }

    #[test]
    fn epub_011_tables_keep_every_cell_in_order_with_ranges() {
        let source = r"<body><table>
            <tr><th>Tree</th><th>Age</th></tr>
            <tr><td>Oak</td><td>300 years</td></tr>
        </table></body>";
        let blocks = convert_xhtml(source).expect("within node budget");
        assert_eq!(blocks.len(), 1);
        let table = &blocks[0];
        assert_eq!(table.kind, BlockKind::Table);
        assert_eq!(table.text, "Tree | Age\nOak | 300 years");

        let extracted: Vec<&str> = table
            .cells
            .iter()
            .map(|range| &table.text[range.clone()])
            .collect();
        assert_eq!(extracted, ["Tree", "Age", "Oak", "300 years"]);
    }

    #[test]
    fn epub_005_malformed_but_recoverable_xhtml_stays_readable() {
        let source = "<body><p>Unclosed paragraph\n   with   odd\tspacing<p>next \
                      <b>bold<i>nested</b> italic</i></p> \
                      <p>&unknownentity &#xZZ; stays literal-safe</p>";
        let blocks = convert_xhtml(source).expect("within node budget");

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
        assert_eq!(
            texts(&convert_xhtml(source).expect("within node budget")),
            ["visible"]
        );
    }

    #[test]
    fn epub_005_hostile_nesting_depth_stays_bounded_without_panic() {
        let deep = format!(
            "{}<p>bottom</p>{}",
            "<div>".repeat(4_000),
            "</div>".repeat(4_000)
        );
        let blocks = convert_xhtml(&deep).expect("within node budget");
        // The innermost paragraph sits far beyond the depth cap and is
        // dropped deterministically instead of overflowing the stack.
        assert!(blocks.is_empty(), "{:?}", texts(&blocks));

        let shallow = format!("{}<p>kept</p>{}", "<div>".repeat(8), "</div>".repeat(8));
        assert_eq!(
            texts(&convert_xhtml(&shallow).expect("within node budget")),
            ["kept"]
        );
    }

    #[test]
    fn sec_009_node_budget_rejects_before_parser_allocation_exactly_at_the_boundary() {
        // The scan counts raw '<' bytes, so plain repetition exercises the
        // policy without needing valid markup.
        let at_limit = "<".repeat(MAX_XHTML_NODES);
        convert_xhtml(&at_limit).expect("exactly the node budget still converts");

        let over = "<".repeat(MAX_XHTML_NODES + 1);
        assert_eq!(
            convert_xhtml(&over),
            Err(XhtmlBoundsError::TooManyNodes {
                nodes: MAX_XHTML_NODES + 1,
                limit: MAX_XHTML_NODES,
            })
        );
    }

    #[test]
    fn sec_009_injected_limits_stop_hostile_chapters_before_any_tree_allocation() {
        let hostile = "<p>".repeat(50);
        assert_eq!(
            convert_xhtml_with_limits(&hostile, 49),
            Err(XhtmlBoundsError::TooManyNodes {
                nodes: 50,
                limit: 49
            })
        );

        // At the injected boundary conversion proceeds normally.
        let blocks = convert_xhtml_with_limits("<p>kept</p>", 2).expect("at budget");
        assert_eq!(texts(&blocks), ["kept"]);

        // The default policy matches the documented EPUB limits table.
        assert_eq!(MAX_XHTML_NODES, 1_000_000);
    }
}
