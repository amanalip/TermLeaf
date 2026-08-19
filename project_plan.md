# TermLeaf Project Plan

**Last updated:** August 19, 2026 at 7:14 PM EDT

## Table of Contents

- [The Idea](#the-idea)
- [Product Boundaries](#product-boundaries)
- [Locked First-Release Features](#locked-first-release-features)
- [What Success Looks Like](#what-success-looks-like)
- [Formats and Platforms](#formats-and-platforms)
- [Technical Direction](#technical-direction)
- [System Architecture](#system-architecture)
- [Document Ingestion](#document-ingestion)
- [EPUB Safety Policy](#epub-safety-policy)
- [Text Model and Layout](#text-model-and-layout)
- [Reader State and Navigation](#reader-state-and-navigation)
- [Terminal Interface](#terminal-interface)
- [Configuration and Persistence](#configuration-and-persistence)
- [Errors and Diagnostics](#errors-and-diagnostics)
- [Accessibility](#accessibility)
- [Testing Strategy](#testing-strategy)
- [Continuous Integration](#continuous-integration)
- [Release and Supply Chain](#release-and-supply-chain)
- [Delivery Roadmap](#delivery-roadmap)
- [When a Feature Is Finished](#when-a-feature-is-finished)
- [Questions Still Open](#questions-still-open)
- [Risks Worth Watching](#risks-worth-watching)
- [References](#references)
- [Keeping the Plan Honest](#keeping-the-plan-honest)

## The Idea

Reading in a terminal should feel intentional, not like opening a text file and
making do. TermLeaf aims for the quiet parts of a good e-reader: crisp pages,
quick movement, a reliable bookmark, and controls that soon become muscle
memory. It should start fast, work offline, and leave the reader's library on
their own machine.

The application will own the reading experience from source text to terminal
cells. That means document positions, line wrapping, navigation, and saved
progress cannot be delegated to a generic text widget. The extra care is worth
it because a reader must keep its place when the terminal changes size.

## Product Boundaries

The first release will concentrate on opening local books and reading them
well. It will not try to become an editor, cloud service, format converter, or
graphical document viewer.

Included in the first release:

- Local plain-text, Markdown, and reflowable EPUB books.
- Paged and continuous full-screen reading modes.
- Semantic text, best-effort inline images, and confirmed external links.
- Hybrid conventional and Vim-style keyboard navigation.
- Smart-case search within the open book.
- Safe reading-position persistence, bookmarks, highlights, and notes.
- A recent-books screen without automatic library scanning.
- Built-in dark, light, high-contrast, monochrome, and Paper themes.
- Detailed reading status and discoverable help.
- Native Linux, macOS, and Windows builds after their release tests pass.

Explicitly outside the first release:

- Book editing or source-document authoring.
- Cloud accounts and cross-device synchronization.
- Digital rights management circumvention.
- Fixed-layout EPUB presentation.
- PDF reflow or page rendering.
- Remote book downloads from inside the application.
- Plugins or an extension runtime.
- A permanent library database before the bookshelf needs indexed queries.

PDF remains separate because it is a fixed-layout graphics format. Text may be
stored in drawing order, fonts may lack usable Unicode maps, and scanned pages
need optical character recognition. Adding a PDF parser would not turn those
documents into reliable reflowable books.

TermLeaf may store highlights and notes about a passage, but it will not modify
the source book or present itself as a document editor.

## Locked First-Release Features

The choices in this section form the first-release product contract. They may
change only through an explicit scope decision recorded in `commit_tracker.md`.

### Feature Summary

| Area | Locked behavior |
| --- | --- |
| Inputs | Local TXT, Markdown, reflowable EPUB 2, and reflowable EPUB 3 |
| Reading flow | Paged by default, with continuous scrolling available at any time |
| EPUB text | Headings, emphasis, strong text, lists, quotes, code, links, separators, and useful table content |
| Images | Attempt every safely enabled decoder, prefer native terminal graphics, then cell rendering, then a caption |
| Navigation | Arrow and paging keys plus familiar Vim-style bindings |
| Search | Forward and backward literal smart-case search with visible matches |
| Position | Automatically save and restore a logical location for every book |
| Bookmarks | Create, name, rename, list, jump to, and delete bookmarks |
| Annotations | Create, color, edit, list, jump to, and delete highlights and notes |
| Home screen | Recent books with reopen, remove-from-recents, clear, and open-path actions |
| Library | No directory scanning or permanent metadata index in the first release |
| Themes | Dark, light, high contrast, monochrome, and Paper |
| Status | Title, chapter, logical location, dynamic page, percentage, clock, reading mode, and temporary messages |
| Links | Show the destination and require confirmation before opening a system browser |
| Help | Searchable or scannable command and key reference inside the application |
| Platforms | Linux, macOS, and Windows after native tests and packaging pass |

### Opening and Returning to Books

TermLeaf accepts a local path on the command line. Starting without a path opens
the recent-books screen. Opening a supported book places the reader at its most
recent valid logical position, or at the beginning when no saved position
exists.

The recent-books screen will:

- Keep a bounded, most-recently-used list.
- Show title and author when trustworthy metadata is available.
- Fall back to a recognizable file name and path when metadata is absent.
- Reopen a selected book.
- Let the reader choose another local path.
- Remove one stale or unwanted entry without deleting the source file.
- Clear the list after confirmation.
- Mark moved, missing, or inaccessible files without repeatedly failing.

It will not crawl folders, watch the filesystem, download metadata, or build a
hidden catalog. A full library index remains a possible post-release feature.

### Reading Modes

Paged mode is the default. One page is the current content viewport after
reserving space for the status line and any visible frame. Page movement uses a
logical anchor, so changing width or theme does not turn the old visual page
number into a bookmark.

Continuous mode scrolls through the same layout model by visual rows. Switching
between modes keeps the first meaningful visible passage anchored. Both modes
support line, page, chapter, table-of-contents, search-result, bookmark, and
annotation jumps.

### Semantic Content

TXT preserves paragraphs and deliberate blank lines after safe decoding.
Markdown and EPUB map into the same document model and retain reading-relevant
structure:

- Headings and section boundaries.
- Paragraphs and explicit line breaks.
- Emphasis, strong text, and inline code.
- Ordered, unordered, and nested lists.
- Quotations and fenced code blocks.
- Links with visible destination handling.
- Horizontal separators.
- Tables reduced to a readable terminal layout or linearized when too narrow.
- Image alt text and captions.

CSS will inform only the small set of semantics TermLeaf deliberately supports.
It will not reproduce browser layout, custom fonts, animation, absolute
positioning, scripts, or fixed-page geometry.

Markdown support will use `pulldown-cmark` and its source offsets rather than
converting Markdown to HTML first. Raw HTML follows the same inert, bounded HTML
path as EPUB content.

### Image Rendering

Images are best effort because terminal capabilities vary sharply. The reader
must never emit several graphics protocols blindly and hope one works. It will
use positive capability evidence or an explicit user override.

The ordered display path is:

1. Honor an explicit protocol or fallback override.
2. Use Kitty graphics after a positive capability query.
3. Use Sixel after positive capability reporting and compatibility checks.
4. Use the iTerm2 inline-image protocol in a known compatible terminal.
5. Use a true-color Unicode half-block rendering inside ordinary cells.
6. Show alt text, a caption, dimensions, and a short failure reason.

`ratatui-image` will integrate images with Ratatui's redraw model. `image` will
decode bounded raster input. `usvg` and `resvg` will rasterize static SVG and
SVGZ without scripts, animation, network access, or host filesystem access.
The `ratatui-image` default features will be disabled so Chafa does not add an
unplanned native runtime and license obligation.

The first release will attempt these bounded formats:

- PNG and a static APNG preview.
- JPEG.
- GIF first frame.
- WebP.
- BMP.
- ICO.
- TIFF.
- PNM.
- TGA.
- QOI.
- DDS.
- OpenEXR.
- Radiance HDR.
- Farbfeld.
- Static SVG and SVGZ.
- Additional formats only when the chosen decoder handles them safely on every
  supported platform and their dependency and license costs have been reviewed.

HEIF, HEIC, JPEG XL, PDF, video, audio, arbitrary attachments, and animated
playback are not promised. AVIF requires a separate native decoding decision
and is not automatically included merely because an encoder feature exists.

Initial image limits:

| Resource | Initial limit |
| --- | ---: |
| Compressed raster input | 32 MiB |
| SVG or SVGZ XML input | 8 MiB |
| Width or height | 16,384 pixels |
| Total decoded pixels | 64 million |
| Decoder allocation budget | 256 MiB |
| Animated preview | First frame only |

Decode, SVG rasterization, resizing, and protocol encoding happen away from the
UI thread. The work queue is bounded, and stale page-image requests can be
discarded. SVG resource resolution accepts only bounded data or canonical EPUB
archive entries. It rejects network URLs, absolute paths, escaping parent paths,
device paths, and host filesystem reads.

### Navigation and Keys

Default controls will support both conventional terminal expectations and a
small Vim-style set. The exact conflict-free map will be tested in Stage 1, but
the feature contract includes:

| Action | Conventional family | Vim-style family |
| --- | --- | --- |
| Previous or next line | Up and Down | `k` and `j` |
| Previous or next page | Page Up and Page Down | Ctrl-B and Ctrl-F |
| Start or end | Home and End | `gg` and `G` |
| Previous or next section | Documented modified arrows | `[` and `]` family |
| Search | Documented command key | `/`, `n`, and `N` |
| Table of contents | Documented command key | A mnemonic single key |
| Bookmark or annotation | Documented command key | Mnemonic single keys |
| Help | F1 | `?` |
| Exit or back | Escape and documented quit key | `q` where unambiguous |

No essential action will require mouse input, AltGr, key-release events, or a
modern terminal keyboard extension. Final bindings must avoid collisions with
text entry in search and note-editing modes.

### Search

In-book search is literal and smart-case:

- A lowercase query matches without case sensitivity.
- A query containing uppercase characters preserves case sensitivity.
- Search moves forward or backward from the current logical position.
- All visible matches are highlighted without moving the saved reading anchor.
- Next and previous result actions wrap only after a clear indication.
- Search history remains local and has a clear action.
- Matches map from normalized search text back to original logical ranges.

Regular expressions, fuzzy body search, and a persistent cross-book index are
outside the first release.

### Bookmarks, Highlights, and Notes

All annotations live in TermLeaf's versioned local state. Source TXT, Markdown,
and EPUB files remain untouched.

Bookmarks support a reader-supplied name and one logical location. Highlights
cover a logical range and use a small accessible color set. Notes attach
editable plain text to a logical range or point.

The annotation view will:

- List bookmarks, highlights, and notes for the current book.
- Show a short passage preview and chapter context.
- Jump to an item without losing the previous position unexpectedly.
- Rename bookmarks.
- Change an allowed highlight color.
- Create and edit note text.
- Delete one item after confirmation.
- Recover gracefully when an edited source book invalidates an old range.

Annotation export, synchronization, sharing, Markdown injection, and EPUB
modification are outside the first release.

### Themes and the Paper View

The first release includes built-in dark, light, high-contrast, monochrome, and
Paper themes. Readers can select a theme in the current session and persist the
choice through TOML configuration. Arbitrary custom palettes are deferred.

The Paper theme will make the content area feel like a page without pretending
the terminal is a graphical typesetter:

- A warm ivory page field.
- Dark charcoal text.
- Muted olive accents that connect to the TermLeaf logo.
- Restrained sepia selection and search highlights.
- A subtle centered page boundary when the terminal is wide enough.
- Comfortable horizontal margins that shrink before content becomes unusable.
- A full-canvas fallback when a distinct page would leave too little room.
- Nearest-color fallbacks for 256-color terminals.
- A contrast-preserving monochrome fallback.

The Paper theme will not change the terminal font, fake paper texture with noisy
characters, or sacrifice contrast for decoration.

### Status and Progress

The detailed status line shows:

- Book title.
- Current chapter or section.
- Logical location.
- Dynamic page within the current layout where meaningful.
- Overall reading percentage.
- Paged or continuous mode.
- Current clock time.
- Temporary confirmations, warnings, search counts, and pending-save state.

Detailed does not mean crowded. Fields collapse in a documented priority order
on narrow terminals, and temporary messages replace lower-priority metrics long
enough to be read. Dynamic page numbers are never persisted as bookmarks.

### External Links

Activating an external link opens a confirmation view that shows the complete
destination. Only an explicit confirmation launches the system browser. The
reader can cancel and leave the URL visible for ordinary terminal selection.

TermLeaf will validate the scheme, treat suspicious or unsupported schemes as
non-openable text, and never follow a link while parsing a book. Internal EPUB
links navigate inside the document without a browser prompt.

### Help and Discoverability

The in-application help view will explain keys by current mode, show available
commands, describe image and accessibility fallbacks, and link each status
indicator to plain language. It must be usable without leaving the book and
must return to the exact logical passage that opened it.

### Platform Promise

The first release intends to provide Linux, macOS, and Windows artifacts. Each
platform earns the promise only after native builds, core tests, PTY journeys,
terminal restoration checks, and clean installation tests pass. A protocol such
as Sixel may be unavailable on a supported platform without making text reading
unsupported; the image fallback chain is part of the platform contract.

## What Success Looks Like

- A common book reaches its first readable page in well under a second on
  representative hardware.
- Moving by line, page, or chapter feels immediate.
- Resizing keeps the same passage in view instead of treating wrapped rows as
  permanent positions.
- Closing the application is safe, and the next session returns to the same
  logical passage.
- Narrow terminals, malformed input, and unsupported files produce clear
  behavior rather than damaged state or an abandoned alternate screen.
- Every essential action works from the keyboard and appears in the help view.
- Installation is short enough to explain clearly and repeatable enough to
  trust.

Initial performance budgets are provisional until fixtures and benchmark
hardware are recorded:

| Measure | Initial budget |
| --- | ---: |
| Warm launch to an empty reader | 150 ms |
| First page of a typical local book | 500 ms |
| Navigation after layout | 50 ms |
| Relayout after resize | 100 ms |
| Save reading state | 50 ms |
| Memory for a typical book | 150 MiB |

These are engineering targets, not public promises. Measurements must include
the machine, operating system, terminal, book size, and build profile.

## Formats and Platforms

### Format Order

| Format | Plan | Reason |
| --- | --- | --- |
| Plain text | First vertical slice | It exposes layout, navigation, and state problems without archive or markup complexity. |
| EPUB 2 and EPUB 3 | First structured format | EPUB provides chapters, metadata, navigation, and reflowable content that match TermLeaf's purpose. |
| Markdown | First release | Its structure maps directly into the shared document model through a source-aware parser. |
| PDF | Not planned for the first release | Reliable reading order, reflow, fonts, images, and OCR form a separate subsystem. |

Plain-text decoding will accept valid UTF-8 and UTF-8 with a byte-order mark.
UTF-16 little-endian and big-endian files will be accepted when a byte-order
mark identifies them. Automatic legacy-encoding detection will wait because a
guessed encoding can silently alter a book.

### Platform Position

Linux, macOS, and Windows are engineering targets because Ratatui and Crossterm
support all three. A platform becomes officially supported only after native
builds, terminal integration tests, installation tests, and release packaging
pass there. Cross-compilation alone is not enough evidence for a terminal app.

Representative terminal coverage should include:

- Windows Terminal on Windows.
- The system Terminal application on macOS.
- At least GNOME Terminal and Konsole on Linux.
- Kitty or WezTerm as a modern protocol-aware terminal.
- A session through SSH.
- A session inside tmux where practical.

The exact supported matrix remains open until the first terminal test harness
is running.

## Technical Direction

TermLeaf will use stable Rust. Rust fits the product because it produces a
single native executable, starts quickly, gives precise control over memory and
I/O, and makes invalid state harder to represent. The planned minimum supported
Rust version is 1.88, which matches the current Ratatui requirement. This value
must be verified when the initial manifest is created.

### Core Stack

Versions in this table reflect the ecosystem review on August 19, 2026. The
manifest should use compatible version requirements, while the committed
`Cargo.lock` records the exact resolved graph.

| Concern | Selection | Why it fits |
| --- | --- | --- |
| Terminal UI | `ratatui 0.30.x` | Mature immediate-mode rendering, custom widgets, a test backend, and broad ecosystem support. |
| Terminal backend | `crossterm 0.29.x` | Pure Rust input and screen control across Linux, macOS, and Windows. |
| Command line | `clap 4.6.x` | Clear help, validation, future subcommands, and shell-completion support. |
| EPUB semantics | `rbook 0.7.x` | Typed EPUB 2 and 3 metadata, manifest, spine, navigation, landmarks, and lazy resources. |
| Archive inspection | `zip 8.6.x` | Exposes member sizes, paths, compression methods, and overlap checks needed before EPUB parsing. |
| XHTML parsing | `scraper 0.27.x` | Tolerant HTML5 parsing through `html5ever` for imperfect real-world chapters. |
| Markdown parsing | `pulldown-cmark 0.13.x` | A mature event stream with source offsets and no required HTML round trip. |
| Terminal images | `ratatui-image 11.x` | Ratatui-aware Kitty, Sixel, iTerm2, and half-block rendering paths. |
| Raster decoding | `image 0.25.x` | Bounded decoding into one normalized bitmap representation. |
| Static SVG | `usvg` and `resvg 0.48.x` | Script-free SVG parsing and portable rasterization under a restricted resolver. |
| Text decoding | `encoding_rs 0.8.x` | Reliable UTF-8 and BOM-identified UTF-16 decoding. |
| Graphemes | `unicode-segmentation 1.13.x` | Prevents clipping and navigation through the middle of a user-perceived character. |
| Cell width | `unicode-width 0.2.x` | Estimates terminal columns for Unicode strings and common emoji sequences. |
| Line breaks | `unicode-linebreak 0.1.x` | Implements Unicode line-break opportunities, including better CJK behavior. |
| Normalization | `unicode-normalization 0.1.x` | Supports canonical text handling and future normalized search. |
| Serialization | `serde 1.x` | Typed configuration and state formats. |
| Configuration | `toml 1.1.x` | Familiar, editable settings. |
| Saved state | `serde_json 1.x` | Simple versioned machine-owned state. |
| Atomic state writes | `tempfile 3.x` | Same-directory temporary files followed by atomic replacement. |
| Platform paths | `directories 6.x` | Native configuration, cache, data, and state locations. |
| Domain errors | `thiserror 2.x` | Matchable errors for expected document and state failures. |
| Application errors | `anyhow 1.x` | Context at I/O, startup, and process boundaries. |

### Deliberately Deferred Crates

| Crate or category | Revisit when |
| --- | --- |
| Tokio or another async runtime | Real concurrent network I/O or many independent asynchronous tasks appear. |
| `tracing` | Parsing, indexing, or support reports need structured file diagnostics. |
| `rusqlite` | The bookshelf needs indexed metadata queries, migrations, or full-text search. |
| `notify` | Reloading changed books or settings has a defined conflict policy. |
| `unicode-bidi` | Right-to-left layout, highlighting, and terminal behavior can be tested together. |
| ICU4X segmentation and casing | Locale-sensitive behavior justifies its data and dependency cost. |
| `hyphenation` | Language detection and dictionary licensing have been settled. |
| `regex` | User-facing regular-expression search is a real requirement. |
| `nucleo-matcher` | A fuzzy chapter or library picker exists. |
| `tantivy` | Multi-book indexed search becomes part of the product. |
| Chafa | A packaged native Unicode renderer proves worth its runtime and LGPL compliance cost. |

No configuration framework, database, file watcher, or async runtime belongs
in the first dependency graph. Handwritten code is smaller and clearer for the
few sources TermLeaf initially has.

## System Architecture

The architecture separates source semantics, logical reader state, visual
layout, and terminal I/O. Parsing a book must not require a terminal, and
testing navigation must not require escape sequences.

```text
src/
|-- main.rs
|-- cli.rs
|-- app/
|   |-- mod.rs
|   |-- action.rs
|   `-- state.rs
|-- document/
|   |-- mod.rs
|   |-- model.rs
|   |-- text.rs
|   `-- epub/
|       |-- mod.rs
|       |-- archive.rs
|       `-- xhtml.rs
|-- layout/
|   |-- mod.rs
|   |-- line_break.rs
|   `-- viewport.rs
|-- reader/
|   |-- navigation.rs
|   |-- position.rs
|   `-- search.rs
|-- persistence/
|   |-- config.rs
|   `-- state.rs
`-- ui/
    |-- mod.rs
    |-- reader.rs
    |-- help.rs
    `-- status.rs
```

### Dependency Direction

```text
CLI and terminal events
        |
        v
Application actions -> application state
        |                    |
        v                    v
Document model         layout and navigation
        |                    |
        `----------> rendered cell model
                             |
                             v
                      Ratatui and Crossterm
```

The document, layout, reader, and persistence modules must not depend on
Ratatui. The UI may translate their results into Ratatui cells and styles. This
keeps the core testable and prevents a widget API from defining book positions.

### Event Loop

The first event loop will remain synchronous:

1. Poll Crossterm with a short timeout.
2. Convert keyboard, resize, focus, and paste events into application actions.
3. Drain completed work from parser or search worker channels.
4. Update application state in one place.
5. Redraw only when state is dirty.
6. Persist changed reading state at controlled checkpoints and on clean exit.

Large parsing or search jobs can run on ordinary worker threads. A generation
number will identify stale work so a result from an old search or layout can be
discarded safely. This provides useful cancellation without an async runtime.

## Document Ingestion

Every format adapter will produce one logical document model. Rendering code
will not know whether a paragraph came from TXT, EPUB, or a future Markdown
file.

```rust
struct Document {
    id: DocumentId,
    metadata: Metadata,
    sections: Vec<Section>,
}

struct Section {
    id: SectionId,
    title: Option<String>,
    blocks: Vec<Block>,
}

enum Block {
    Heading { level: u8, content: Vec<Inline> },
    Paragraph(Vec<Inline>),
    Quote(Vec<Block>),
    List(Vec<ListItem>),
    Code(String),
    Separator,
}

enum Inline {
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Code(String),
    Link { target: String, content: Vec<Inline> },
}
```

The exact Rust types may change during implementation, but the boundaries are
intentional. Source structure should survive long enough to support headings,
lists, links, search, and stable positions without carrying a browser DOM into
the reader.

### Plain-Text Path

1. Open the file without reading beyond configured size limits.
2. Detect a supported byte-order mark.
3. Decode through `encoding_rs` when UTF-16 is explicitly marked.
4. Otherwise require valid UTF-8.
5. Normalize CRLF and CR line endings into logical newlines.
6. Preserve paragraph breaks and intentional blank lines.
7. Convert paragraphs into the shared document model.

Extremely long logical lines must be processed in bounded chunks. Invalid
encoding should report the file and reason rather than replacing bytes without
telling the reader.

### EPUB Path

1. Open the file as an immutable local source.
2. Inspect the archive with `zip` before semantic parsing.
3. Enforce archive, entry, compression, and path limits.
4. Let `rbook` resolve the package, metadata, manifest, spine, and navigation.
5. Visit linear spine resources in canonical reading order.
6. Parse chapter XHTML with the HTML5 parser behind `scraper`.
7. Ignore scripts, styling that cannot map to the terminal, and remote loads.
8. Convert meaningful block and inline structure into TermLeaf's model.
9. Preserve source identifiers needed by EPUB links and saved locations.
10. Report unsupported encryption or fixed layout clearly.

EPUB control documents such as `container.xml`, OPF, and NCX remain XML. Book
chapters need tolerant HTML parsing because real EPUB files often contain XHTML
mistakes that browsers can recover from safely.

## EPUB Safety Policy

An EPUB is an untrusted ZIP archive containing XML, XHTML, images, fonts, and
links. A valid extension does not make it safe. The loader must reject input
that would consume unreasonable memory, CPU time, disk space, or parser depth.

### Initial Limits

These limits are application policy rather than EPUB specification limits.
They should become configurable only if real books provide a good reason.

| Resource | Initial limit |
| --- | ---: |
| Compressed EPUB file | 256 MiB |
| ZIP members | 10,000 |
| Advertised total uncompressed size | 512 MiB |
| Container, OPF, NCX, or navigation file | 16 MiB |
| Single XHTML chapter | 32 MiB |
| Compression ratio | 100:1 with a small-file exception |
| XML depth | 256 levels |
| XML nodes | 1,000,000 |

### Required Protections

- Count actual decompressed bytes instead of trusting archive metadata alone.
- Reject absolute paths, NUL bytes, escaping parent paths, and ambiguous member
  names.
- Reject overlapping ZIP entries, symlinks, unsupported compression methods,
  and encrypted resources.
- Read resources directly from the archive rather than unpacking a book to
  disk.
- Disable DTDs and external entity resolution.
- Never execute scripts or active content.
- Never fetch remote URLs while opening or rendering a book.
- Treat `file:`, `javascript:`, and external links as inert metadata unless a
  later, explicit policy says otherwise.
- Keep images and fonts lazy. Text extraction must not decode them by default.
- Detect fixed-layout EPUB metadata and explain that the format is unsupported.
- Keep the inspected bytes stable between preflight and semantic parsing so
  the file cannot change underneath the checks.

`rbook` is the semantic layer, not the security boundary. Its resource helpers
can allocate based on input, so TermLeaf must enforce bounded reads around the
archive.

## Text Model and Layout

The logical document remains independent of screen width. A layout pass turns
logical blocks into visual rows for one viewport and records how those rows map
back to source positions.

### Layout Order

1. Start with canonical logical text and semantic spans.
2. Split only at grapheme-cluster boundaries.
3. Find Unicode line-break opportunities.
4. Measure candidate fragments in terminal cells.
5. Wrap to the available content width.
6. Apply indentation and block styling without losing source offsets.
7. Record a mapping from each visual span to its logical range.
8. Cache layout by section, width, and relevant display settings.

Ratatui's `Paragraph` widget can display simple text, but it will not own this
process. Its wrapped rows are visual output, not durable positions.

### Unicode Commitments

- Navigation and clipping will not split grapheme clusters.
- Width calculations will use terminal cells rather than Unicode scalar count
  or UTF-8 byte length.
- CJK line-break opportunities will be considered from the first layout
  implementation.
- Search will run over logical text, never visually reordered or wrapped rows.
- Normalized search will keep an explicit map back to original byte ranges.
- Ambiguous-width behavior may become a reader setting because terminals and
  fonts do not always agree.

Full bidirectional layout is deferred until logical positions, wrapping,
visual reordering, highlighting, and real terminal behavior can be tested as
one feature. TermLeaf must not claim complete right-to-left support before that
work is finished.

## Reader State and Navigation

A page number is not a stable bookmark in a reflowable reader. Changing the
terminal width changes the number and contents of pages.

The persisted position will identify logical content:

```rust
struct ReadingPosition {
    document_id: DocumentId,
    section_id: SectionId,
    block_index: usize,
    byte_offset: usize,
}
```

The final type may include a short content fingerprint so a position can be
recovered when the same file changes slightly. The first implementation should
prefer a deterministic document identifier based on canonical path plus stable
file identity or content metadata, with privacy implications documented.

Navigation actions will include:

- Next and previous visual line.
- Next and previous page.
- Next and previous chapter or section.
- Start and end of the current section.
- Start and end of the document.
- Jump to a table-of-contents entry.
- Jump to a search result.
- Return from help or another temporary view without losing the reading anchor.

Every action will update logical state first. The viewport then derives the
visible rows from that state.

## Terminal Interface

Ratatui will render an application-owned state model through Crossterm. The
first screen needs only a reading viewport, a restrained status line, help, and
clear error presentation.

### Input Rules

- Every essential action has a conventional keyboard binding.
- Arrow keys, Page Up, Page Down, Home, and End work alongside letter keys.
- Mouse support is optional and cannot be the only route to an action.
- Essential bindings do not depend on key-release events, AltGr, unusual
  modifier combinations, or modern keyboard protocols.
- Paste events are accepted only in modes that expect text input.
- Key bindings become configurable only after the default interaction has been
  tested and documented.

### Terminal Lifecycle

Raw mode, alternate screen, cursor visibility, mouse capture, paste mode, and
keyboard enhancements must be restored after:

- Normal exit.
- A handled application error.
- Ctrl-C or another supported termination signal.
- A panic where cleanup remains possible.
- A failed startup after terminal initialization begins.

Terminal cleanup is correctness, not polish. Integration tests must prove that
TermLeaf does not leave the user's shell damaged.

## Configuration and Persistence

Configuration precedence will be simple and visible:

```text
built-in defaults < config.toml < explicit command-line options
```

No layering framework is needed for these three sources. The code will apply
only command-line values the reader actually supplied.

### Storage Locations

`directories::ProjectDirs` will choose platform-native locations:

| Data | Preferred location |
| --- | --- |
| Editable settings | Configuration directory |
| Reading positions and recent books | State directory, with local data as fallback |
| Rebuildable indexes and caches | Cache directory |

### State Format

Machine-owned state will be versioned from its first schema:

```json
{
  "schema_version": 1,
  "recent_books": [],
  "positions": {}
}
```

Config and state will not share one serialized structure. Settings have human
editing and precedence rules. State has migration, corruption, and durability
rules.

### Atomic Save Sequence

1. Create a uniquely named temporary file in the destination directory.
2. Serialize the complete new state.
3. Flush the writer and call `sync_all()` on the file.
4. Atomically persist or rename it over the destination.
5. Synchronize the parent directory where the platform and durability policy
   support it.
6. Keep the old state if any earlier step fails.

Tests will deserialize the newly written file and simulate interrupted writes.
A valid but unsupported schema version must produce a recoverable message, not
silent data loss.

## Errors and Diagnostics

Expected failures will use typed domain errors. Examples include unsupported
format, malformed archive, unsafe resource, invalid encoding, fixed-layout
EPUB, unsupported state version, and missing file.

`anyhow` will add context at executable boundaries, while `thiserror` will keep
domain failures matchable. Raw debug chains will not be printed over an active
alternate screen. Reader-facing errors should say:

- What failed.
- Which path or resource was involved when safe to reveal.
- Why TermLeaf stopped or skipped it.
- What the reader can try next.

Persistent logging is deferred. When startup, parsing, or indexing becomes hard
to diagnose, `tracing` can write structured diagnostics to a file only when the
reader opts in. Routine logs must never corrupt the terminal screen.

## Accessibility

Terminal UI libraries do not provide a mature semantic accessibility tree.
TermLeaf should be honest about that limitation and offer behavior that remains
usable outside a visually rich full-screen mode.

Required practices:

- Keep all operations keyboard-accessible.
- Do not communicate meaning through color alone.
- Respect terminal foreground and background defaults where possible.
- Provide high-contrast and monochrome options.
- Honor `NO_COLOR` or an equivalent documented setting.
- Avoid animation and unnecessary redraws.
- Keep errors and status text available long enough to read.
- Provide noninteractive commands or a plain-text output mode where useful.
- Test real screen readers and platform terminals before claiming support.

Automated cell snapshots can verify text and style placement, but they cannot
prove screen-reader usability. Manual assistive-technology testing belongs in
the release process.

## Testing Strategy

Testing will follow the architecture from pure logic toward real terminals.
Most behavior should be proven without spawning a terminal process.

### Model and Unit Tests

- Format detection and decoding.
- Archive limits and rejected ZIP structures.
- EPUB spine order, metadata, navigation, and malformed resources.
- Document-model conversion.
- Grapheme-safe wrapping and width invariants.
- Navigation algebra, including moving forward and back to the same anchor.
- Search offsets and normalized-to-original mappings.
- State round trips, migrations, and corruption handling.
- Configuration precedence.

### Property Tests

Add `proptest` when layout and navigation exist. Useful properties include:

- No rendered row exceeds the requested cell width.
- Layout never splits a grapheme cluster.
- Valid input never causes a panic or infinite loop.
- Resizing preserves the logical reading anchor.
- State serialization followed by deserialization preserves supported values.
- Moving to the next page always makes progress unless already at the end.

### Render Tests

Render into Ratatui's test backend or an application-owned cell grid. Keep a
small set of reviewed `insta` snapshots for:

- A normal `80x24` terminal.
- A narrow terminal such as `40x10`.
- Empty, short, long, malformed, and large documents.
- ASCII, combining marks, CJK, emoji, tabs, and control characters.
- Reader, help, search, error, and resize states.
- Default, high-contrast, and monochrome presentation.

Snapshots complement direct assertions. They do not replace checks for source
positions, widths, navigation, or persistence.

### Terminal Protocol Tests

Use `vt100` to feed emitted ANSI bytes into a deterministic terminal model and
assert final cells, cursor position, clearing behavior, alternate-screen exit,
and restored modes.

Use `portable-pty` for a small number of end-to-end journeys:

- Launch, open a book, navigate, and exit.
- Resize while reading.
- Handle Ctrl-C and normal termination.
- Restore the terminal after a panic path where testable.
- Distinguish TTY from piped input.

PTY tests need strict timeouts, deterministic locale and `TERM`, child cleanup,
and low parallelism. Native jobs must run them because emulation cannot certify
platform terminal behavior.

### Fuzzing and Performance

Add `cargo-fuzz` with the first untrusted parsers. Targets should cover ZIP
preflight, EPUB control files, XHTML conversion, text decoding, and state-file
loading.

Use Criterion after representative fixtures exist. Benchmarks should cover
launch, opening, first layout, resize, page navigation, literal search, and
state serialization across tiny, typical, large, malformed, CJK, emoji, and
right-to-left samples where supported.

## Continuous Integration

Every proposed change should eventually run:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo deny check
```

Native Linux, macOS, and Windows jobs will build and run core tests for every
platform TermLeaf intends to support. `cargo-nextest` can be added when suite
runtime, process isolation, or CI reporting makes it worthwhile. Standard
`cargo test --doc` remains necessary because Nextest does not run doctests.

Scheduled work should include advisory refreshes, longer fuzz runs, dependency
updates, and selected performance checks. Wall-clock benchmarks from shared CI
runners should inform investigation rather than act as hard gates.

## Release and Supply Chain

`cargo-dist` is the planned release builder after native release builds work by
hand. It can produce archives, checksums, installers, manifests, and release
automation without making a package-publication workflow the center of a
desktop application.

Release requirements:

- Commit `Cargo.lock` and build with `--locked`.
- Build on native runners for supported desktop platforms.
- Pin GitHub Actions to reviewed commit hashes.
- Keep workflow permissions read-only unless one job needs more.
- Generate artifacts only from protected tags and commits.
- Attach checksums and verify them during installation smoke tests.
- Provide the corresponding source revision and build instructions.
- Generate third-party license notices with `cargo-about`.
- Use `cargo-deny` for advisories, licenses, sources, and banned dependencies.
- Consider an SBOM and artifact attestations before the first public release.

The existing repository contains GPLv3 text. Before Rust initialization, the
project must state whether it uses `GPL-3.0-only` or `GPL-3.0-or-later`, add the
chosen SPDX expression to `Cargo.toml`, and place clear copyright and license
notices where readers and distributors can find them.

Dependency checks do not cover fonts, icons, sample books, dictionaries, or
test corpora. Those assets need their own provenance and license review.

## Delivery Roadmap

### Stage 0: Establish the Rust Foundation

Work:

- Create the Cargo package with the chosen license expression and Rust version.
- Add Ratatui, Crossterm, Clap, error handling, and the first test setup.
- Implement terminal setup and guaranteed restoration.
- Define the application action and state loop.
- Add formatting, Clippy, tests, and `cargo-deny` to CI.
- Record target platforms, terminals, and provisional performance hardware.

Exit gate:

- A minimal application opens and exits cleanly on native target platforms.
- Normal exit, startup failure, Ctrl-C, and a controlled panic restore the
  terminal.
- CI passes from a locked dependency graph.

### Stage 1: Complete the Plain-Text Reading Loop

Work:

- Decode supported plain-text files into the document model.
- Implement grapheme-aware, cell-width-aware layout.
- Render the reading viewport and status line.
- Implement paged and continuous modes.
- Navigate by line, page, start, and end with hybrid default keys.
- Keep a stable logical anchor through resize.
- Add the built-in themes, including the responsive Paper view.
- Report missing files, invalid encoding, and unusable terminal sizes.

Exit gate:

- A reader can open a representative text book, read, resize, and navigate
  without losing the passage.
- Model, property, render, and basic PTY tests cover the journey.
- Provisional interaction and memory budgets are measured.

### Stage 2: Add Structured Books and Images

Work:

- Implement bounded ZIP preflight and archive policy errors.
- Integrate `rbook` for package, spine, metadata, and navigation semantics.
- Convert chapter XHTML into the shared document model.
- Parse Markdown directly into the shared document model.
- Add chapter and table-of-contents navigation.
- Detect encrypted and fixed-layout books.
- Decode bounded raster and SVG resources away from the UI thread.
- Implement protocol detection, half-block fallback, and caption fallback.
- Build a licensed corpus of EPUB 2, EPUB 3, malformed, large, and hostile
  fixtures.
- Fuzz archive, XML, XHTML, and conversion boundaries.

Exit gate:

- Representative EPUB 2 and EPUB 3 books follow canonical reading order.
- Unsafe archives fail within defined resource limits.
- Malformed but recoverable XHTML remains readable.
- Unsupported encryption and fixed layout receive specific messages.
- Image failures never block surrounding text or damage terminal output.

### Stage 3: Make Reading Dependable

Work:

- Define versioned configuration and state schemas.
- Save and restore logical reading positions atomically.
- Add recent books and smart-case in-book search.
- Add bookmarks, highlights, notes, and their management view.
- Add help and complete keyboard coverage.
- Introduce monochrome and high-contrast presentation.
- Expand native terminal and accessibility testing.
- Benchmark large books and remove avoidable relayout work.

Exit gate:

- Interrupted writes do not destroy previous state.
- Resizing and reopening return to the same logical passage.
- Search results map correctly to wrapped output.
- Claimed terminals pass native integration tests.

### Stage 4: Refine the Reading Desk

Work:

- Refine recent-book history and metadata presentation.
- Refine annotation recovery when source books move or change.
- Verify external-link confirmation and browser launching on each platform.
- Keep automatic library indexing outside the first release.
- Test the common paths with readers other than the author.
- Finish user, troubleshooting, and contributor guides.

Exit gate:

- Returning to a book is quick and understandable.
- Added storage has migration and recovery tests.
- Accessibility and performance targets hold up under real use.

### Stage 5: Ship It

Work:

- Finalize the supported platform and terminal matrix.
- Produce native release artifacts through `cargo-dist`.
- Generate checksums, license notices, and source references.
- Run clean-machine installation and upgrade tests.
- Publish known limitations, especially Unicode and accessibility boundaries.
- Complete the release checklist and rollback procedure.

Exit gate:

- A new reader can install TermLeaf, open a book, and start reading by following
  the published instructions exactly.
- Release artifacts are reproducible enough to investigate differences and are
  traceable to the tagged source and lockfile.
- Every promised platform passes its native smoke tests.

## When a Feature Is Finished

A feature earns **Complete** in the tracker when:

- Its reader-visible behavior and important failure cases are clear.
- The implementation respects module and dependency boundaries.
- Unit or model tests cover its logic.
- Integration tests cover the boundary it crosses.
- Untrusted-input code has limits and fuzz coverage appropriate to its risk.
- Formatting, Clippy, tests, dependency policy, and builds pass.
- Performance remains inside an agreed budget or the exception is documented.
- Reader-facing documentation and trackers match the delivered behavior.
- Decisions that could puzzle a future contributor appear in the decision log.

## Questions Still Open

| Question | Why it matters | Target stage |
| --- | --- | --- |
| Is the license `GPL-3.0-only` or `GPL-3.0-or-later`? | The manifest and distributed notices need an exact SPDX expression. | Stage 0 |
| Which OS versions and terminals are promised? | Release claims need native evidence and maintenance boundaries. | Stage 0 |
| What exact hybrid key map avoids text-entry conflicts? | Defaults shape daily use and future configuration. | Stage 1 |
| How should a document identity survive moves or edits? | Saved positions need stability without collecting unnecessary private data. | Stage 1 |
| Which optional image decoders pass the security and platform review? | Broad format attempts must not add fragile native dependencies or unsafe allocation. | Stage 2 |
| Which recoverable EPUB errors become warnings? | Permissive parsing helps readers but must not hide unsafe input. | Stage 2 |
| What level of right-to-left support can be promised? | Bidi layout, terminal shaping, search, and highlights must agree. | Stage 3 |
| Does a local library index improve actual use? | A database should solve demonstrated retrieval problems. | Stage 4 |
| Which package channels should be maintained? | Every channel adds release and upgrade obligations. | Stage 5 |

## Risks Worth Watching

| Risk | What could go wrong | Response |
| --- | --- | --- |
| Archive resource exhaustion | A small EPUB expands into excessive memory or CPU work. | Preflight and count bounded reads before semantic parsing. |
| Fragile saved positions | Resize or content changes return readers to the wrong passage. | Persist logical anchors and test relayout independently of page rows. |
| Unicode width disagreement | Terminal output clips, drifts, or leaves stale cells. | Use grapheme and width crates, expose ambiguous-width policy, and test real terminals. |
| Incomplete bidi behavior | Right-to-left text displays or highlights incorrectly. | Defer support claims until the full logical-to-visual pipeline is tested. |
| Terminal input ambiguity | AltGr, non-Latin layouts, or modifier combinations fail. | Keep essential bindings simple and test native keyboard paths. |
| Broken terminal restoration | The shell remains in raw mode or hides the cursor. | Centralize lifecycle cleanup and cover normal, error, signal, and panic paths. |
| Parser dependency defects | A trusted crate accepts unsafe or malformed input badly. | Keep limits at TermLeaf's boundary, run `cargo-deny`, fuzz inputs, and update deliberately. |
| Snapshot complacency | Updated snapshots approve a behavioral regression. | Pair snapshots with invariants and require focused review. |
| Platform claims outrun testing | A binary builds but behaves poorly in a real terminal. | Require native interaction and installation tests before support claims. |
| Dependency growth | Convenience crates slow builds and widen the audit surface. | Add a crate only for a measured need and review features with `cargo tree`. |
| License gaps | A release omits notices or includes an incompatible asset. | Review the resolved graph and non-Cargo assets before every release. |

## References

Research and versions were checked on August 19, 2026. Primary specifications,
project documentation, and tool references take precedence over summaries in
this plan.

### Terminal UI and Input

- [Ratatui documentation](https://docs.rs/ratatui/latest/ratatui/)
- [Ratatui backend comparison](https://ratatui.rs/concepts/backends/comparison/)
- [Ratatui Paragraph documentation](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Paragraph.html)
- [Ratatui text-wrapping discussion](https://github.com/ratatui/ratatui/issues/293)
- [Ratatui accessibility discussion](https://github.com/ratatui/ratatui/issues/2610)
- [Crossterm documentation](https://docs.rs/crossterm/latest/crossterm/)
- [Crossterm event model](https://docs.rs/crossterm/latest/crossterm/event/)
- [Crossterm tested terminals](https://github.com/crossterm-rs/crossterm#tested-terminals)

### EPUB and Document Parsing

- [EPUB 3.3 specification](https://www.w3.org/TR/epub-33/)
- [EPUB Open Container Format](https://www.w3.org/TR/epub-33/#sec-ocf)
- [EPUB security and privacy](https://www.w3.org/TR/epub-33/#security-privacy)
- [rbook documentation](https://docs.rs/rbook/latest/rbook/)
- [rbook open options](https://docs.rs/rbook/latest/rbook/epub/struct.EpubOpenOptions.html)
- [zip documentation](https://docs.rs/zip/latest/zip/)
- [ZipArchive documentation](https://docs.rs/zip/latest/zip/read/struct.ZipArchive.html)
- [ZipFile path validation](https://docs.rs/zip/latest/zip/read/struct.ZipFile.html#method.enclosed_name)
- [scraper documentation](https://docs.rs/scraper/latest/scraper/)
- [html5ever documentation](https://docs.rs/html5ever/latest/html5ever/)
- [encoding_rs documentation](https://docs.rs/encoding_rs/latest/encoding_rs/)
- [pulldown-cmark documentation](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/)
- [ratatui-image documentation](https://docs.rs/ratatui-image/latest/ratatui_image/)
- [image format support](https://docs.rs/image/latest/image/codecs/index.html#supported-formats)
- [image decoder limits](https://docs.rs/image/latest/image/struct.Limits.html)
- [resvg documentation](https://docs.rs/resvg/latest/resvg/)
- [usvg options](https://docs.rs/usvg/latest/usvg/struct.Options.html)
- [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [iTerm2 image protocol](https://iterm2.com/documentation-images.html)

### Unicode and Search

- [Unicode segmentation](https://docs.rs/unicode-segmentation/latest/unicode_segmentation/)
- [Unicode terminal width](https://docs.rs/unicode-width/latest/unicode_width/)
- [Unicode line breaking](https://docs.rs/unicode-linebreak/latest/unicode_linebreak/)
- [Unicode normalization](https://docs.rs/unicode-normalization/latest/unicode_normalization/)
- [Unicode bidirectional algorithm](https://docs.rs/unicode-bidi/latest/unicode_bidi/)

### Configuration and Persistence

- [Clap documentation](https://docs.rs/clap/latest/clap/)
- [directories documentation](https://docs.rs/directories/latest/directories/)
- [Serde documentation](https://serde.rs/)
- [TOML crate documentation](https://docs.rs/toml/latest/toml/)
- [serde_json documentation](https://docs.rs/serde_json/latest/serde_json/)
- [tempfile persistence documentation](https://docs.rs/tempfile/latest/tempfile/struct.NamedTempFile.html#method.persist)
- [thiserror documentation](https://docs.rs/thiserror/latest/thiserror/)
- [anyhow documentation](https://docs.rs/anyhow/latest/anyhow/)

### Testing, Policy, and Releases

- [Rust testing guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Proptest documentation](https://docs.rs/proptest/latest/proptest/)
- [Insta documentation](https://insta.rs/docs/)
- [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [vt100 documentation](https://docs.rs/vt100/latest/vt100/)
- [portable-pty documentation](https://docs.rs/portable-pty/latest/portable_pty/)
- [Criterion documentation](https://criterion-rs.github.io/book/)
- [cargo-deny documentation](https://embarkstudios.github.io/cargo-deny/)
- [RustSec advisory database](https://rustsec.org/)
- [cargo-dist documentation](https://axodotdev.github.io/cargo-dist/book/)
- [GitHub Actions security guidance](https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions)
- [Cargo license manifest fields](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields)
- [GNU GPL frequently asked questions](https://www.gnu.org/licenses/gpl-faq.html)

## Keeping the Plan Honest

This plan should change when evidence changes the route, not simply because a
date passed. Update `implementation_tracker.md` as work moves or stalls. Update
`commit_tracker.md` whenever a commit changes behavior or settles an important
question. Refresh the timestamp on any document whose meaning changes.

Crate versions, security advisories, and platform behavior will move. Review
them when the Cargo manifest is created, before each release, and whenever a
dependency update changes the resolved graph.
