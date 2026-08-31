# Commit Tracker

**Last updated:** August 30, 2026

## Table of Contents

- [Purpose](#purpose)
- [Update Process](#update-process)
- [Pending Commit](#pending-commit)
- [Commit History](#commit-history)
- [Design Decision Log](#design-decision-log)

## Purpose

Git can show which lines moved. This tracker explains why they moved. It keeps
each meaningful change beside the decisions a future contributor would
otherwise have to reconstruct.

## Update Process

1. Describe active work under **Pending Commit**.
2. Add choices with lasting consequences to the **Design Decision Log**.
3. Compare the entry with the staged diff before committing.
4. Move the entry to **Commit History**, add the intended subject and time,
   then check both against Git history after the commit lands.

## Pending Commit

### Record Phase 2 gate evidence

**Intended subject:** `docs: record Phase 2 gate evidence`

- Record all six passing Linux jobs at revision `1ca9a17` without combining
  evidence from earlier revisions.
- Preserve nine incomplete gate cases as Planned, including manual `IMG-018`.
- Document native, format, resource, worker, optional-fuzz, and dependency limits
  without promoting unsupported terminal tuples.

### Stabilize generated EPUB fixtures

**Intended subjects:**

- `fix: stabilize hostile EPUB fixture`
- `fix: stabilize generated EPUB corpus`

- Remove Python/zlib-version variance from generated EPUB bytes by storing every
  authored ZIP member.
- Regenerate all affected archives and update their registered hashes, sizes,
  and parameters.
- Retain the same semantic, malformed, and hostile payloads and parser evidence.

### Align release scope with Linux-only support

**Intended subject:** `docs: limit release scope to Linux`

- Make Linux the sole first-release platform in public and implementation plans.
- Keep Kitty and Sixel native acceptance while removing iTerm2, macOS, Windows,
  and ConPTY execution from the active queue without renumbering tasks.
- Preserve portable implementation code and historical platform evidence without
  making unsupported compatibility claims.

### Limit required test environments to Linux

**Intended subject:** `ci: limit test matrix to Linux`

- Run every hosted profile family only on Ubuntu 24.04.
- Make `ENV-LINUX-PTY` the sole required phase-gate environment while retaining
  macOS and Windows IDs as Deferred historical tuples.
- Regenerate the case registry and all cumulative phase-gate manifests from the
  Linux-only source policy.

### Implement terminal graphics probing and PTY lifecycle evidence

**Intended subject:** `feat: probe terminal graphics capabilities`

- Send one bounded Kitty, Sixel-identifying XTGETTCAP, and iTerm2 query packet
  before the first frame while preserving unrelated Crossterm events.
- Freeze strict complete-response parsing, one-shot backend precedence, and
  cell/caption fallback for malformed, partial, absent, or negative evidence.
- Add protocol-aware PTY journeys for selection, fallback, replacement,
  scrolling, exclusivity, missing geometry, cleanup, and restoration.
- Write the protocol-neutral `IMG-018` procedure and keep native visual
  acceptance and hosted rows unclaimed until those environments run.

Validation is recorded in
`testreport.md#implement-terminal-graphics-probing-and-pty-lifecycle-evidence`.

### Record native transport completion

**Intended subject:** `docs: record native transport completion`

- Mark Task 11 complete after `7e5cf55` closes every deterministic hardening
  finding left by the native transport foundation.
- Record measured Sixel geometry, cooperative cancellation, bounded PNG and
  base64 allocation, exact worker accounting, explicit edge fallback, and
  failure-safe lifecycle cleanup.
- Keep Tasks 12 through 14 open: no active capability probing, hosted native
  lifecycle journey, or real-terminal `IMG-018` result is promoted by this work.

Validation is recorded in
`testreport.md#harden-native-graphics-execution`.

### Add deterministic robustness evidence

**Intended subject:** `test: add deterministic robustness corpus`

- Move all `FUZZ-*` IDs to optional weekly discovery and prohibit them and
  default duration tables from required profiles and frozen phase gates.
- Generate and validate 29 small authored fixtures, including one sample for
  every enabled raster decoder, with hashes, provenance, SPDX licenses,
  parameters, properties, and served case links.
- Harden text decoding against malformed UTF-8, UTF-16, unsupported UTF-32,
  limit overflow, false-positive unmarked UTF-16 detection, arbitrary fixed-seed
  bytes, and bounded mutations.
- Preserve valid UTF-8 content, including interior U+FEFF; only the one leading
  UTF-8 BOM selected by the encoding marker is removed.

Validation is recorded in `testreport.md#complete-the-first-three-phase-2-tasks`.

### Define the remaining implementation queue

**Intended subject:** `docs: define the remaining implementation queue`

- Add `tasks left.md` with 60 stable, dependency-ordered tasks grouped under
  Phases 0 through 5; completed numbers stay visible so requests such as
  “implement the next 10” remain unambiguous.
- Replace mandatory duration-based fuzzing in the human plan and quality policy
  with deterministic malformed-input, exact-boundary, fixed-seed property,
  hostile-corpus, and fixed-mutation evidence.
- Retain coverage-guided fuzzing as optional scheduled or pre-release discovery
  and require every discovered defect to become a deterministic regression.
- Leave the executable test catalog and generated manifests unchanged in this
  documentation batch. Queue their coordinated migration first so registry
  validation never accepts a stale or hand-edited manifest.
- Correct Phase 2 status: structured ingestion, bounded image decoding, workers,
  fallback rendering, and TOC behavior exist, while native graphics emission,
  active capability probing, deterministic robustness evidence, and external
  gate evidence remain.

Validation:

- `git diff --check` passes.
- `python3 tools/case_registry.py check` passes.
- The new queue contains exactly 60 continuously numbered tasks.
- Cargo and native checks are not applicable because no source, executable test,
  dependency, fixture, or generated manifest changes.

### Harden loose image resource access

**Intended subject:** `fix: harden loose image resource access`

- Replace path canonicalization followed by ambient reopen with one persistent
  capability directory and relative opens.
- Reject Windows device/drive spellings on every host and prove static plus
  concurrent symlink swaps cannot expose an outside file.
- Scope the Unix-only EPUB decoy fixture so Windows Clippy sees no unused local.

### Complete Phase 2 development

**Intended subjects:**

- `feat: finish structured resource handling`
- `feat: integrate bounded image rendering`
- `docs: record phase 2 development status`

- Links carry inert destinations and source ranges; EPUB internal targets map
  to validated logical navigation points rather than browser actions.
- Every EPUB control-document class is structurally gated before semantics;
  successful reads remain archive-only with no extraction or sidecars.
- `resvg`/`usvg` run with default features and external resolvers disabled;
  SVGZ is streamed under its actual XML limit before static rasterization.
- Image backend selection requires positive evidence. Explicit negative
  evidence is a typed error (DD-031); cell and caption paths are deterministic.
- Background work uses two workers, an eight-request queue, and a 64 MiB
  in-flight input budget with immediate rejection (DD-032). Generations own
  cancellation and stale-result policy.
- Wide contents navigation uses a side panel; narrower screens retain the
  full-screen overlay.

Decisions:

- **DD-031:** Explicit image overrides may bypass absent detection, but may not
  contradict explicit negative capability evidence. Contradiction returns one
  typed error; automatic protocol emission still requires positive evidence.
- **DD-032:** Worker limits are two threads, eight waiting requests/completions,
  and 64 MiB of queued/running inputs. Submission never blocks the UI.

## Commit History

### Harden native graphics execution

**Completed:** August 30, 2026

**Commit subject:** `fix: harden native graphics execution`

**Revision:** `7e5cf55`

Choices with lasting consequences:

- Sixel output is pixel-addressed only when Crossterm reports nonzero terminal
  pixel and cell dimensions; missing geometry produces a typed caption fallback.
- Native image fitting, PNG compression, Kitty/iTerm2 base64 chunking, and Sixel
  generation checkpoint cancellation without retaining a full base64 copy.
- PNG writes reject before crossing the 16 MiB native-output limit, and worker
  completion accounting includes retained object, vector, and Arc allocations.
- A native image must fit wholly inside the current content viewport before its
  escape placement is emitted; partial placement renders a warning caption and
  becomes eligible again after scrolling fully into view.
- Attempted native IDs remain cleanup-tracked after write or flush failures.
  Capability probing and real-terminal acceptance remain separate work.

### Wire image ingestion into books

**Completed:** August 22, 2026 at 7:25 PM EDT

**Commit subject:** `feat: wire image ingestion into books`

**Revision:** `22969d3`

Choices with lasting consequences:

- Images enter the document model as caption placeholder blocks
  (`BlockKind::Image`) whose canonical text is exactly `[image: alt]` or
  `[image]`; the model never holds pixels. Decoding stays lazy behind a new
  `ImageResource` reference (container member key plus declared size, or an
  unfetchable marker), so hostile books cannot force decode work by merely
  being opened.
- Mid-flow images split the enclosing flow in both XHTML and Markdown, and
  the Markdown split reopens the original block kind so trailing words are
  never dropped. Raw-HTML `<img>` remains completely inert.
- EPUB resolution is chapter-relative and archive-bounded: strict percent
  decoding, scheme rejection, dot-segment merging that cannot escape the
  package root, and existence checks against the preflighted archive only.
  Unresolvable targets stay visible as captions but are marked unfetchable.
- Markdown destinations classify under the same policy: plain relative
  paths keep fetchable references (byte length unknown until read);
  absolute, parent-escaping, colon-bearing (scheme or drive), and escaping
  targets block.

## Commit History

### Add bounded raster image decoding

**Completed:** August 22, 2026 at 6:14 PM EDT

**Commit subject:** `feat: add bounded raster image decoding`

**Revision:** `ef32bce`

Implements the decode core of the Phase 2 image slice. Choices with lasting
consequences:

- `image` 0.25 joins the manifest with default features off and exactly the
  locked format set enabled (PNG, JPEG, GIF, WebP, BMP, ICO, TIFF, PNM, TGA,
  QOI, DDS, OpenEXR, Radiance HDR, Farbfeld). The plan's format table is the
  contract; adding a decoder later requires the same dependency and license
  review.
- Limits live in `ImageLimits` and are enforced in policy order: input bytes
  gate before parsing, header-only dimension reads gate before any pixel
  allocation, pixels then a per-family allocation ceiling (4 B/px baseline;
  PNG/TIFF 8; HDR 12; OpenEXR 16) gate before decoding. This keeps hostile
  or corrupt files from allocating anything beyond header reads.
- Format resolution is extension-first with content magic winning when
  present, mirroring `DEC-TEST-001`: TGA (no magic) decodes only through its
  declared extension; mislabeled-but-signed files follow their signature.
- Animation previews resolve to the first frame only, matching the locked
  "first frame" promise; SVG/SVGZ stay out of this module until the vector
  slice lands with its restricted resolver.
- `deny.toml` gains one documented advisory exception (`paste`,
  RUSTSEC-2024-0436, unmaintained without known vulnerability) because the
  locked OpenEXR feature pulls it through `exr -> pulp`; revisit when exr
  drops pulp.

### Add table of contents navigation

**Completed:** August 22, 2026 at 1:25 AM EDT

**Commit subject:** `feat: add table of contents navigation`

**Revision:** `9d09e27`

Changes:

- `Action::ShowToc` joins the registry bound to both `o` and `F2`
  (`DD-019` family: one mnemonic single key plus a conventional function
  key); help lists both automatically.
- The contents overlay opens only over an open book, lands its cursor on
  the current section, scrolls long lists from a clamped window, and
  labels untitled sections by stable ordinal.
- Up/Down move the cursor, Confirm jumps the reading anchor to the
  selected section start and confirms with a tick-lifetime message,
  Escape returns, and help stays reachable with exact return stacking.
- Reader navigation stays inert inside the overlay, so the hidden anchor
  can never move while browsing contents.

Decisions:

- **DD-030:** The table of contents is an overlay view carrying only its
  return target; the cursor is session state seeded from the current
  reading section each time it opens. Jumping reuses the existing
  validated section-start step so TOC jumps cannot produce anchors that
  line/page navigation cannot reproduce.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 140 library
  plus 9 CLI plus 15 document-I/O plus 14 render plus 6 property plus 14
  native PTY Rust tests, doctests, cargo-deny, and diff checks passed
  locally.
- Hosted rows for this revision remain to be recorded on push.

### Complete semantic content and Markdown

**Completed:** August 22, 2026 at 12:40 AM EDT

**Commit subject:** `feat: complete semantic content and markdown`

**Revision:** `3ae3ae6`

Changes:

- Extend the document model (`src/document/model.rs`) with list items
  (nesting depth plus ordering flag), quotes, verbatim code blocks,
  separators, and tables carrying row-major cell ranges; add validated
  inline decorations (emphasis, strong, code, link) that never alter the
  canonical text positions they decorate.
- Teach XHTML conversion the full semantic set: em/strong/code/link
  inline roles with innermost-wins nesting, ordered and unordered lists
  whose nested lists become deeper sibling items, blockquotes, literal
  preformatted code, rules, and tables with per-cell ranges. Whitespace
  collapsing now tracks per-byte roles so decorations survive collapsing
  exactly.
- Add the Markdown adapter (`src/document/markdown.rs`) over
  pulldown-cmark's bounded event stream: source-aware parsing maps
  headings, paragraphs, lists, quotes, fenced and indented code, rules,
  GFM tables, and inline roles into the shared model while raw HTML and
  remote references stay completely inert. `.md`/`.markdown` join the
  case-insensitive extension table; the shared 32 MiB budget applies
  boundary-exactly before any parse.
- Upgrade layout for semantics: spans subdivide at decoration boundaries;
  list markers render with hanging indents at marker width and per-depth
  numbering that restarts after non-list blocks; quote bars prefix every
  row; code blocks emit one verbatim row per line with grapheme-safe hard
  splits; tables align columns when their natural width fits and
  linearize through ordinary wrapping when it does not, keeping every
  cell in order either way. Inter-row newlines ride as spans so ranges
  alone reconstruct canonical bytes exactly.
- Carry inline roles through viewport cells to the reader renderer:
  emphasis italic, strong bold, code distinct, links underlined, each
  distinguishable by attribute alone in `NO_COLOR` sessions.

Decisions:

- **DD-029:** Inline semantics live beside the text, not inside it: the
  canonical string and every position stay byte-stable while sorted
  non-overlapping decoration spans name roles per range. Nested roles
  flatten innermost-wins; link targets are deliberately not stored yet,
  so links are maximally inert until the links slice adds navigation;
  fenced language tags likewise wait with `MD-005`. Tables keep pipe
  delimiters and newline rows in canonical form so search and positions
  see plain readable lines.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 137 library
  plus 9 CLI plus 15 document-I/O plus 14 render plus 6 property plus 14
  native PTY Rust tests, doctests, cargo-deny, and diff checks passed
  locally.
- Hosted rows for this revision remain to be recorded on push.

### Harden the structured ingestion boundaries

**Completed:** August 21, 2026 at 10:22 PM EDT

**Commit subject:** `feat: harden structured ingestion boundaries`

**Revision:** `437ea4c`

Changes:

- Add a markup-node budget to XHTML conversion (`src/document/xhtml.rs`):
  a byte scan counts `<` openings before the HTML5 tree builder allocates,
  the inclusive policy limit is one million openings per chapter matching
  the EPUB limits table, and an injectable-limit variant gives exact
  Boundary Method evidence (at the limit converts, one past rejects)
  without multi-megabyte test inputs. Rejection is the new typed
  `DocumentError::ChapterTooComplex`, naming path, member, count, and
  limit; the existing recursion cap still bounds the walk itself.
- Split EPUB ingestion into a public staged `EpubSnapshot`
  (`src/document/epub.rs`): `open` reads the source once, preflights every
  archive boundary, and closes the handle; `build` resolves package,
  spine, navigation, and chapter semantics over only the inspected
  immutable bytes. The one-call `load_epub_file` path composes both stages.
- Prove byte stability with integration journeys (`tests/document_io.rs`):
  after inspection the source is overwritten with a different complete
  book, truncated to zero bytes, appended with garbage, renamed away,
  deleted outright, and on Unix swapped for a decoy symlink; each build
  still returns the originally inspected title and passage (`EPUB-010`,
  `EPUB-016`). Rename/delete halves run on every platform because the
  source handle is closed before `open` returns.
- Extend `EPUB-005` locations with the node-budget tests; leave `SEC-009`
  open with its chapter-side half landed and compensating controls named.

Decisions:

- **DD-028:** Chapter structure is bounded by counting `<` openings on raw
  bytes rather than DOM nodes: every element, comment, and processing
  instruction consumes at least one opening while plain text never does,
  so the count bounds tree growth without parsing anything. The budget is
  inclusive at exactly one million. Control documents (container, OPF,
  NCX, nav) keep their existing actual-byte preflight limits plus
  `rbook`'s non-recursive pull parser as compensating controls; extending
  structural gates over them would require re-resolving package paths
  TermLeaf deliberately delegates, so it waits for an explicit decision
  under `SEC-009`.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 116 library
  plus 9 CLI plus 13 document-I/O plus 14 render plus 6 property plus 14
  native PTY Rust tests, doctests, cargo-deny, and diff checks passed
  locally; `cargo clean` removed 4,052 files (1.2 GiB).
- Hosted rows for this revision remain to be recorded on push.

### Start the structured book ingestion

**Completed:** August 21, 2026 at 8:19 PM EDT

**Commit subject:** `feat: start structured book ingestion`

**Revision:** `bed99a8`

Changes:

- Add the bounded ZIP preflight layer (`DD-008`,
  `src/document/archive.rs`): one host-independent canonical key per member
  (backslash unification, dot-segment resolution, trailing dot/space
  stripping, parent/NUL/colon rejection), inclusive limits for compressed
  size, member count, advertised expansion, control resources, chapters,
  and compression ratio with a small-file exception; typed rejections for
  encrypted flags, symlinks, unsupported methods, overlapping regions,
  dishonest metadata, truncation/corruption, and CRC failures; actual
  decompressed bytes counted for every control and chapter resource while
  images and fonts stay lazy.
- Add `rbook`-backed EPUB semantics (`src/document/epub.rs`): package,
  metadata, spine, manifest fallbacks, and navigation resolve over the same
  inspected bytes through a shared immutable byte handle; linear-only spine
  order builds multi-section documents; TOC labels name chapter sections;
  encryption.xml presence and fixed-layout metadata receive specific typed
  messages before any chapter decodes.
- Add tolerant XHTML conversion (`src/document/xhtml.rs`) behind the HTML5
  tree builder (`scraper`): headings h1-h6, paragraphs, list items as
  paragraphs, `<br>` breaks, entity decoding, script/style/head exclusion,
  and a deterministic recursion cap against hostile nesting.
- Extend the document model to multiple sections
  (`Document::from_sections`) with cross-section tiling validation and a
  `Heading { level }` block kind; layout rows now carry section-qualified
  block ownership so wrapping spans every section correctly.
- Wire `.epub` into extension-first detection and the unified loader;
  misleading EPUB content fails with typed archive/package errors after
  the gate while `.txt` behavior stays unchanged.
- Ship deterministic committed fixtures FX-EPUB2/FX-EPUB3 via
  `tools/make_epub_fixtures.py` with recorded SHA-256 provenance.
- Resolve DEC-TEST-003 policy concretely in code: ratio = declared vs
  compressed×100 above 64 KiB uncompressed, inclusive boundaries, zero-byte
  entries exempt, aggregate expansion enforced separately.

Decisions:

- **DD-027:** The preflight reads the whole source into memory once under
  the compressed-size boundary and shares it immutably between the archive
  checks and `rbook`, guaranteeing inspected-byte stability (EPUB-010/016
  groundwork) without re-opening the file. Chapter bytes always flow
  through TermLeaf's bounded reader rather than `rbook`'s unbounded
  resource helpers.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 114 library
  plus 9 CLI plus 11 document-I/O plus 14 render plus 6 property plus 14
  native PTY Rust tests, doctests, cargo-deny, and diff checks passed
  locally; real Gutenberg EPUBs parsed with correct titles and section
  counts.
- Hosted rows for this revision remain to be recorded on push.

### Close the Phase 1 gate evidence

**Completed:** August 21, 2026 at 7:15 PM EDT

**Commit subject:** `test: close the Phase 1 gate evidence`

Changes:

- Complete the closeable `phase-gate-1` members. Native PTY journeys now
  cover the full reader key matrix (Up/Down, PageUp/PageDown, Home/End, F1,
  Escape versus Alt chords), flow-control paging, bracketed-paste
  inertness with multiline/control/oversized payloads, resize transients
  through a tiny geometry and back to the same anchor, and locale variants
  (`C`, `en_US.UTF-8`) rendering identical Unicode.
- Extract the terminal event filter so focus, mouse, resize, release, and
  paste events are provably inert while prefix state survives inert
  traffic (`term_007`, universal).
- Add a deterministic property suite (`tests/properties.rs`) over a seeded
  xorshift generator: row width bounds, grapheme integrity, anchor
  survival across resize sequences, page progression plus exact inverse
  semantics, resize-interleaved navigation, and action-sequence state
  validity.
- Add the `tests/render.rs` integration target and activate `pr-render`:
  Paper collapse order cell-by-cell, a three-color-mode by five-viewport
  matrix over reader/help states, exact true-color role values,
  theme-switch anchor preservation across all five themes, status field
  collapse by first-drop widths with message lifetime restore, redraw
  stability, help reachable from Recent books/Reader/Themes/itself, and
  render-layer Unicode placement claims.
- Resolve `DEC-TEST-001` (DD-025): extension-first, case-insensitive `.txt`
  detection; other or missing extensions fail pre-terminal with one typed
  message; `.txt` content still decodes strictly (misleading pairs covered
  both directions).
- Escape C0 control bytes and DEL in failing-path diagnostics through caret
  notation so hostile names cannot inject terminal sequences.
- Fix previous-page inversion found by the properties: the backward step is
  now the smallest content row whose unclamped forward step lands exactly
  on the current page.
- Make help reachable from the theme overlay with exact return stacking.
- Pin ambiguous-width characters to the narrow measurement; add read-only
  source and immutable-open integration journeys; add right-to-left sample
  journeys at five widths.
- Document manual KEY/LAY procedures and forward ownership for cross-phase
  gate members (DD-026) in `manual_procedures.md`.
- Scope the paste journey away from ConPTY after its input pipeline was
  shown to consume bracketed-paste markers (`db27a0f`).

Decisions:

- **DD-025:** Detection is extension-first and case-insensitive. Phase 1
  ships only `.txt`; Markdown and EPUB extend the table in their phases.
  Content validity is enforced after the extension gate. Diagnostics escape
  control bytes through the same caret notation the reader uses.
- **DD-026:** Frozen gate members whose owning features land later are
  owned forward and do not block the Phase 1 exit claim: KEY-005 and NAV-008
  (search/note entry, Phase 3), NAV-009 TOC/annotation halves and NAV-013
  non-line/page jumps (Phases 2-3), LAY-009 tables and LAY-010 code blocks
  (Phase 2), THEME-007 selection/search/link colors (later phases), THEME-008
  images (Phase 2), STATUS-007 failed-save state (Phase 3), ERR-003
  note-content half (Phase 3), PROP-005..PROP-009 (owning feature phases),
  and the human-terminal/font-dependent halves of KEY-001, KEY-002, KEY-006,
  LAY-013, LAY-014 (release native matrix rows).

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 97 library
  plus 8 CLI plus 4 document-I/O plus 14 render plus 6 property plus 14
  native PTY Rust tests, doctests, cargo-deny, and diff checks passed
  locally; `cargo clean` removed 3,537 files (819.3 MiB).
- Hosted run `32535725291` passed all eight jobs on `db27a0f`, evidencing
  ENV-LINUX-PTY, ENV-MAC-PTY, and ENV-WIN-PTY.

## Commit History

### Complete the plain-text reading loop

**Completed:** August 21, 2026 at 5:31 PM EDT

**Commit subject:** `feat: complete the plain-text reading loop`

Changes:

- Add `src/persistence`: bounded startup configuration loading from the
  platform config directory (`serde`, `toml` 1.1.x, `directories` 6.x from
  the sanctioned Core Stack), with `XDG_CONFIG_HOME` relocation on every
  platform for hermetic harnesses.
- Apply configuration precedence in `process::run`:
  built-in default < `config.toml` theme < explicit `--theme` CLI option;
  the CLI validates slugs and documents the option in `--help`.
- Detect terminal color capability once at launch (`COLORTERM`,
  `TERM`) and adapt themes: exact RGB for true color, an exact
  nearest-squared-distance xterm-256 mapping (6×6×6 cube plus grayscale,
  fixed points verified) for 256-color terminals, and the attribute-only
  fallback for unknown terminals or `NO_COLOR`.
- Add native PTY render journeys: open a book, navigate (Down, PageDown,
  Home, `G`), round-trip help with Escape, complete the `gg` prefix across
  PTY reads, quit with full restoration; and a configured-theme journey
  asserting the selection cursor/applied marker plus confirmation message.
- Add integration evidence that files above the byte limit fail before
  terminal setup with the typed message (sparse-file fixture) and that
  startup never rewrites `config.toml`.
- Fix three defects exposed by the PTY journeys: character keys now drop
  SHIFT before binding matching so `G`/`?` fire on real terminals; the event
  loop owns one persistent `KeyMapper` so multikey prefixes survive between
  events; reader navigation/mode actions are inert outside the Reader view.
- Surface temporary status messages on non-reader screens instead of
  dropping them, keeping confirmations visible after theme changes.
- Let typed document errors reach the diagnostic unchanged instead of
  wrapping them in a generic "could not open book" context layer.
- Register all new tests in the case registry; mark `CFG-001`–`CFG-003`,
  `TXT-008`, and `THEME-002` Implemented and extend location-only evidence
  for `KEY-001`, `KEY-003`, and `THEME-005`.
- Allow `BSD-3-Clause` (`encoding_rs` bundled WHATWG data) and `MPL-2.0`
  (`option-ext` via `directories`) in `deny.toml`.

Decisions:

- **DD-022:** Configuration precedence is built-in default <
  `config.toml` < explicit `--theme`. A missing, unreadable, wrong-typed,
  or malformed file falls back to defaults without blocking startup, and an
  unrecognized theme slug resolves to Paper. Typed configuration errors are
  deferred to the Phase 3 CFG cases. A non-empty `XDG_CONFIG_HOME`
  relocates settings on every platform so PTY/CLI harnesses stay hermetic;
  otherwise `directories::ProjectDirs` picks native locations.
- **DD-023:** Output color capability is detected once per launch:
  `COLORTERM` containing `truecolor`/`24bit` selects exact RGB; else `TERM`
  containing `256color` selects nearest-xterm-256 output computed by exact
  squared-RGB distance over the cube and grayscale ramp; anything else
  renders terminal-default attributes only. `NO_COLOR` still forces the
  attribute fallback regardless of detection. Detection is conservative by
  design: unproven capability never receives RGB values.
- **DD-024:** Character-key bindings match without SHIFT because terminals
  deliver capitals as `Char + SHIFT`; Ctrl/Alt semantics are untouched. The
  terminal event loop owns one persistent `KeyMapper` for the whole
  session, making the timer-free prefix policy real outside unit tests.
  Reader navigation and mode actions apply only in the Reader view so help,
  themes, and future overlays can never move the hidden reading anchor.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 88 library
  plus 6 CLI plus 9 native PTY Rust tests, doctests, cargo-deny, and diff
  checks passed locally.
- `cargo clean` removed all generated build output (2,327 files,
  596.5 MiB).
- Hosted environment rows, manual KEY/LAY procedures, render-profile
  snapshot review, and the phase-gate exit run remain open Phase 1 work.

### Start the plain-text reading loop

**Completed:** August 21, 2026 at 11:08 AM EDT

**Commit subject:** `feat: start the plain-text reading loop`

Changes:

- Add Phase 1 core dependencies from the sanctioned Core Stack:
  `encoding_rs`, `unicode-segmentation`, `unicode-width`,
  `unicode-linebreak`, and `thiserror`.
- Add `src/document`: a shared logical model (canonical text, block tiling,
  validated positions) plus the bounded plain-text path with BOM detection,
  strict UTF-8, marked UTF-16 decoding, newline normalization, and
  paragraph/blank-line preservation.
- Add `src/layout`: grapheme-safe, cell-width-aware wrapping at Unicode
  line-break opportunities, span-to-source mapping, tab/control safe
  rendering, and viewport slicing.
- Add `src/reader`: one validated logical anchor, paged/continuous modes,
  line/page/start/end/section navigation with clamped boundaries, and
  floored progress helpers.
- Extend the key map with the hybrid conventional/Vim reader bindings and a
  timer-free `gg` prefix policy; help renders every registered binding.
- Add `src/ui/theme` (five semantic-role themes, tested Paper contrast,
  `NO_COLOR` fallback), `src/ui/status` (priority collapse order, UTC clock,
  tick-lifetime messages), and reader/theme-selection/too-small rendering.
- Register all new tests in the case registry; mark unit-layer cases
  Implemented and record location-only evidence for partially covered IDs.

Decisions:

- **DD-019:** Phase 1 key map fixes line (`j`/`k`, arrows), page
  (`PgUp`/`PgDn`, `Ctrl-B`/`Ctrl-F`), start/end (`Home`/`End`, `gg`/`G`),
  section (`{`/`}`), mode (`p`/`c`), themes (`t`), help (`F1`/`?`), back
  (`Esc`), quit (`q`/`Ctrl-C`). A lone `g` opens a prefix that any other key
  cancels after mapping that key normally; no timers. Initial `DEC-TEST-010`
  resolution, subject to PTY keyboard evidence.
- **DD-020:** Status collapse order drops clock, dynamic page, title,
  chapter, hint, then location before the protected percent+mode pair;
  messages replace lower-priority fields for eight key events. Initial
  `DEC-TEST-011` resolution pending render review.
- **DD-021:** TXT size limit starts at 32 MiB inclusive, checked on metadata
  and again on a guarded read. Partial `DEC-TEST-012` input; config/state/
  query limits remain open.

### Distinguish ConPTY host controls

**Completed:** August 20, 2026 at 1:49 AM EDT

**Commit subject:** `test: distinguish ConPTY host controls`

Changes:

- Keep the strict no-control assertion for Unix pre-terminal failures.
- On Windows, reject TermLeaf's terminal setup sequences while allowing the
  unavoidable ConPTY process envelope.
- Retain the alternate-screen state assertion on every platform.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 28 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- `cargo clean` removed all generated build output.
- Exact Windows behavior is verified by the CI run triggered by this commit.

### Filter echoed ConPTY negotiation

**Completed:** August 20, 2026 at 1:45 AM EDT

**Commit subject:** `test: filter echoed ConPTY handshake`

Changes:

- Remove both the ConPTY cursor query and its cooked-mode echo from captured
  application output.
- Rebuild the terminal model from the filtered transcript on Windows.
- Assert stable help structure without coupling the test to argv display-name
  behavior.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 28 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- `cargo clean` removed all generated build output.
- The triggered CI run supplied exact Windows follow-up evidence.

### Stabilize Windows CLI assertions

**Completed:** August 20, 2026 at 1:40 AM EDT

**Commit subject:** `test: isolate ConPTY negotiation output`

Changes:

- Give Clap a platform-independent `termleaf` display name.
- Exclude ConPTY's startup cursor negotiation from captured application output
  after the harness answers it.
- Preserve the no-control-sequences contract for errors raised before terminal
  initialization.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 28 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- `cargo clean` removed all generated build output.
- The triggered CI run supplied exact Windows follow-up evidence.

### Complete the ConPTY harness fix

**Completed:** August 20, 2026 at 1:35 AM EDT

**Commit subject:** `fix: answer ConPTY startup query`

Changes:

- Answer ConPTY's startup cursor-position request so the hosted child can begin
  execution.
- Use a concrete Windows terminal-state marker to keep all-target Clippy strict
  without suppressing unit-value diagnostics.
- Retain the first CI rerun's exact failure evidence for the follow-up fix.

Validation:

- Formatting, Clippy with warnings denied, registry freshness, 28 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- `cargo clean` removed all generated build output.
- The triggered CI run supplied exact Windows follow-up evidence.

### Fix Windows CI regressions

**Completed:** August 20, 2026 at 1:26 AM EDT

**Commit subject:** `fix: restore Windows CI compatibility`

Changes:

- Compile the `anyhow::bail` test import only on Unix, matching every call site.
- Restore warnings-as-errors compatibility for the Windows all-target Clippy job.
- Preserve the minimal Windows system environment required by `CreateProcessW`
  and ConPTY while keeping user data paths isolated.
- Create redirected test directories before launch and include captured output
  in PTY timeout diagnostics.

Validation:

- Formatting, Clippy with warnings denied, 28 Rust tests, doctests, cargo-deny,
  and diff checks passed locally.
- `cargo clean` removed all generated build output.
- Exact Windows validation is delegated to the CI rerun triggered by this commit.

### Complete Phase 0 implementation

**Completed:** August 20, 2026 at 1:02 AM EDT

**Commit subject:** `feat: complete Phase 0 implementation`

Changes:

- Define every first-release view identity and its exclusive focus owner without
  implementing later-phase screen behavior early.
- Open and retain one read-only source handle, and reject unreadable files before
  terminal initialization.
- Support raw-mode Ctrl-C on every target and catchable external `SIGINT` on
  POSIX through the shared quit action.
- Generalize the PTY target for Unix and Windows/ConPTY and verify the documented
  shell launch baseline.
- Replace broad later-feature test mappings with four exact foundation cases and
  require every Phase 0 case to carry implementation evidence.
- Mark Phase 0 implementation complete while keeping hosted platform evidence
  separate from development progress.

Validation:

- Registry freshness, formatting, Clippy with warnings denied, 28 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- PTY child, reader, and timeout cleanup is bounded and fails the test process
  rather than hanging or detaching work.
- `cargo clean` removed all generated build output.

### Continue the Rust foundation

**Completed:** August 20, 2026 at 12:44 AM EDT

**Commit subject:** `test: add Phase 0 manifests and PTY harness`

Changes:

- Generate and validate all 332 case records, exact execution profiles, and
  cumulative Phase 0 through Phase 5 gate memberships.
- Register Phase 0 environments and the complete planned fixture inventory.
- Isolate CLI and PTY process tests behind case-owned paths, minimal environment,
  fixed dimensions, deterministic terminal parsing, and hard deadlines.
- Prove Linux normal exit, Ctrl-C input, pre-terminal failure, active handled
  error, and recoverable panic restoration through real PTYs.
- Suppress panic diagnostics until terminal unwinding completes, and define
  fixed native CI runners plus Unix PTY jobs.
- Keep Phase 0 open for required hosted, macOS, Windows, MSRV, signal-policy,
  and arbitrary-initial-mode evidence.

Validation:

- Registry freshness, formatting, Clippy with warnings denied, 23 Rust tests,
  doctests, cargo-deny, and diff checks passed locally.
- PTY tests compared kernel terminal attributes and modeled ANSI modes before
  and after normal, error, panic, and Ctrl-C paths.
- `cargo clean` removed all generated build output.

### Start the Rust foundation

**Completed:** August 20, 2026 at 12:15 AM EDT

**Commit subject:** `feat: start Rust foundation`

Changes:

- Initialize the Rust 1.88 package with a locked dependency graph and an exact
  `GPL-3.0-only` license expression.
- Add the foundation application state, focus model, action registry, CLI, and
  deterministic Ratatui shell.
- Add a terminal lifecycle guard that rolls back partial setup and restores all
  changed modes after normal return, cleanup failure, or unwinding.
- Add unit, render, and CLI process tests plus locked cross-platform CI and
  cargo-deny policy.
- Mark Phase 0 in progress without claiming unfinished registry, profile, PTY,
  signal, or native-platform evidence.

Validation:

- Formatting, Clippy with warnings denied, 13 Rust tests, doctests, and
  cargo-deny passed on Linux with Rust 1.97.1.
- `cargo clean` removed the generated local build output.
- Rust 1.88 and native PTY/platform evidence remain assigned to CI and later
  Phase 0 work rather than being claimed from this environment.

### Define quality, testing, and UI implementation standards

**Completed:** August 20, 2026 at 12:05 AM EDT

**Commit subject:** `docs: define implementation standards`

Changes:

- Add `code_quality.md` as the enforceable implementation and review contract.
- Cover architecture, types, errors, hostile input, terminal cleanup,
  concurrency, persistence, privacy, performance, dependencies, and tests.
- Require honest per-commit validation and `cargo clean` after complete local
  Rust validation cycles.
- Add `testcases.md` with stable IDs, exact test layers and profiles, fixture and
  environment registries, change selection, phase gates, and completion rules.
- Catalog feature, security, privacy, terminal, accessibility, performance,
  supply-chain, property, fuzz, native, and release evidence.
- Audit the catalog in four passes and replace unresolved oracles with explicit
  Blocked decision records.
- Add `ui_mockups.md` with detailed responsive ASCII screens, focus/input modes,
  overlays, accessibility, safety, render guidance, and phase ownership.
- Add stable UI interaction cases for open path, links, selection, history,
  themes, long values, tiny terminals, confirmations, images, help, and errors.
- Reconcile the roadmap and tracker with the standards while retaining six
  phases and strengthening their exact cumulative gates.
- Link all standards from the README, project plan, and implementation tracker.

Validation:

- Four independent test-framework reviews and two UI/roadmap synchronization
  reviews were incorporated.
- The catalog defines 332 unique case IDs with no duplicates or dangling exact
  references.
- Markdown tables, code fences, local links, and table-of-contents anchors
  validate across all eight root documents.
- `ui_mockups.md` contains ASCII only.
- The plan, tracker, test framework, and UI specification each contain exactly
  Phase 0 through Phase 5 in the same order.
- Required Cargo commands agree, Markdown files contain no em dash characters,
  and `git diff --check` passes.

### Refine the planned Paper theme

**Completed:** August 19, 2026 at 10:16 PM EDT

**Commit subject:** `docs: refine Paper theme`

Changes:

- Define semantic Paper colors for the page, canvas, text, controls, selection,
  and search.
- Specify responsive margin and page-boundary collapse behavior.
- Preserve source image colors while styling their surrounding presentation.
- Require contrast, non-color indicators, logical-anchor preservation, and
  render coverage across terminal color capabilities and viewport sizes.

Validation:

- All seven planned text color pairings exceed the 4.5:1 contrast floor; the
  lowest calculated ratio is 5.14:1.
- Local Markdown links resolve, changed headings retain their table-of-contents
  entries, and Markdown files contain no em dash characters.
- `git diff --check` passes.

### Establish test evidence and a local book corpus

**Completed:** August 19, 2026 at 7:32 PM EDT

**Commit subject:** `docs: establish test reporting`

Changes:

- Add `testreport.md` as the required per-commit validation record.
- Record exact source, size, and SHA-256 details for six ignored Gutenberg test
  inputs.
- Link test reporting from the README, project plan, and implementation tracker.
- Add all six delivery phases and their exit-gate summaries to the operational
  implementation tracker.
- Require `cargo clean` after each complete local Rust validation cycle.

Validation:

- All three EPUB archives pass `unzip -t` without member errors.
- All three text books are identified as UTF-8 text.
- `git check-ignore -v` confirms that every downloaded book remains ignored.
- Local Markdown links resolve, Markdown files contain no em dash characters,
  and `git diff --check` passes.

### Lock the first-release reader experience

**Completed:** August 19, 2026 at 7:18 PM EDT

**Commit subject:** `docs: lock first-release feature set`

Changes:

- Record the agreed formats, reading modes, navigation, search, recents,
  annotations, themes, status details, links, and platform intent.
- Define a capability-driven image path with bounded raster and SVG decoding.
- Add acceptance boundaries that distinguish first-release features from later
  library, export, animation, and search work.
- Align the README and implementation tracker with the locked product contract.

Validation:

- Current image rendering, raster decoding, SVG, and Markdown parser references
  were checked against their primary documentation.
- Every Markdown file retains a table of contents and readable timestamp.
- Markdown files contain no em dashes or common filler buzzwords.
- `git diff --check` completed without whitespace errors.

### Turn the Rust research into the build plan

**Completed:** August 19, 2026 at 6:55 PM EDT

**Commit subject:** `docs: define detailed Rust project plan`

Changes:

- Replace unresolved stack questions with the researched Rust architecture.
- Detail document ingestion, EPUB limits, Unicode layout, persistence, tests,
  native CI, release gates, and remaining risks.
- Link the plan to primary specifications and crate documentation.
- Bring the implementation tracker in line with the chosen direction.

Validation:

- Key Ratatui, rbook, ZIP, EPUB, cargo-deny, and cargo-dist references were
  checked against their current primary documentation.
- Every Markdown file retains a table of contents and readable timestamp.
- Markdown files contain no em dashes or common filler buzzwords.
- `git diff --check` completed without whitespace errors.

### Give TermLeaf a recognizable mark

**Completed:** August 19, 2026 at 6:27 PM EDT

**Commit subject:** `feat: add TermLeaf logo`

Changes:

- Add a scalable logo that joins a terminal, leaf, and open book.
- Display the logo at the top of the README.
- Record the visual direction and completed logo work.

Validation:

- The SVG passed XML validation and rendered successfully at 512 pixels.
- The README image path resolves to the new asset.
- Markdown files contain no em dashes or whitespace errors.

### Give the project notes a distinct voice

**Completed:** August 19, 2026 at 6:22 PM EDT

**Commit subject:** `docs: sharpen project writing`

Changes:

- Rewrite the README around the experience TermLeaf wants to create.
- Turn the project plan from a generic checklist into a practical story of how
  the reader will take shape.
- Make implementation notes direct, specific, and easy to scan.
- Keep every Markdown file free of em dashes.

Validation:

- All four Markdown files retain a table of contents and readable timestamp.
- Local document links and table-of-contents anchors were checked.
- No em dashes or common filler buzzwords remain in the Markdown files.
- `git diff --check` completed without whitespace errors.

### Repository planning foundation

**Completed:** August 19, 2026 at 5:52 PM EDT

**Commit subject:** `docs: add project planning foundation`

Changes:

- Add stack-neutral ignore rules for local, generated, and sensitive files.
- Add project planning and implementation tracking documents.
- Add a commit-oriented change and design decision log.
- Expand the README with links to project documentation.

Validation:

- Markdown links and table-of-contents anchors were verified.
- `git diff --check` completed without whitespace errors.
- Ignore rules were checked against representative local files and build
  directories.
- The Git working tree contains only the five intended file changes.

## Design Decision Log

### DD-001: Keep initial planning technology-neutral

**Date:** August 19, 2026 at 5:48 PM EDT

**Status:** Accepted

The repository does not yet declare a language, framework, or package manager.
The plan stays focused on the reading experience until the first-release scope
gives us a sound reason to choose an implementation stack.

### DD-002: Track documentation beside source code

**Date:** August 19, 2026 at 5:48 PM EDT

**Status:** Accepted

Planning, implementation status, and commit context sit at the repository root.
That keeps them visible, reviewable, and close to the code they describe.

### DD-003: Do not ignore dependency lockfiles

**Date:** August 19, 2026 at 5:48 PM EDT

**Status:** Accepted

Lockfiles are intentionally absent from `.gitignore`. Once the project chooses
a package manager, its lockfile will help make builds repeatable.

### DD-004: Write like a person building TermLeaf

**Date:** August 19, 2026 at 6:20 PM EDT

**Status:** Accepted

Project notes should sound specific to TermLeaf, vary their rhythm, and tell a
reader why the work matters. We will avoid canned summaries, inflated claims,
and em dashes. Trackers can stay structured, but their wording should remain
plain and concrete.

### DD-005: Build the mark from the product idea

**Date:** August 19, 2026 at 6:25 PM EDT

**Status:** Accepted

The logo combines the three parts of the name and purpose: a terminal window,
a living leaf, and an open book. Flat shapes and a restrained charcoal, lime,
and warm-white palette keep it readable at icon size and independent of the
viewer's light or dark theme. SVG is the source format so the mark can scale
without introducing generated binary files.

### DD-006: Build the reader in Rust with Ratatui and Crossterm

**Date:** August 19, 2026 at 6:50 PM EDT

**Status:** Accepted

Stable Rust, Ratatui, and Crossterm give TermLeaf a fast native executable, an
application-owned state model, deterministic rendering tests, and a credible
path across Linux, macOS, and Windows. The core will use a synchronous event
loop and ordinary worker threads rather than carrying an async runtime before
the product has asynchronous work.

### DD-007: Own logical document positions and layout

**Date:** August 19, 2026 at 6:50 PM EDT

**Status:** Accepted

TermLeaf will parse each format into its own semantic document model and map
logical positions to visual terminal rows during layout. Ratatui widgets will
display the result but will not define wrapping, navigation, search offsets, or
saved positions. This keeps a reader at the same passage when the viewport
changes.

### DD-008: Put a bounded archive layer in front of EPUB parsing

**Date:** August 19, 2026 at 6:50 PM EDT

**Status:** Accepted

`rbook` will handle EPUB 2 and 3 semantics after a direct `zip` preflight checks
member paths, counts, sizes, compression ratios, overlap, encryption, and
supported methods. TermLeaf will read resources from the archive without
unpacking them and will keep active or remote content inert.

### DD-009: Keep early infrastructure small and local

**Date:** August 19, 2026 at 6:50 PM EDT

**Status:** Accepted

TOML settings, versioned JSON state, platform-native directories, and atomic
same-directory writes cover the first release. A database, file watcher,
configuration framework, persistent logger, and async runtime will wait for a
measured need.

### DD-010: Lock a rich but local first-release reader

**Date:** August 19, 2026 at 7:14 PM EDT

**Status:** Accepted

The first release will read local TXT, Markdown, and reflowable EPUB books in
paged or continuous mode. It includes hybrid keys, smart-case search, recent
books, saved positions, bookmarks, highlights, notes, detailed status, built-in
themes, confirmed external links, and native Linux, macOS, and Windows targets.
It will not scan libraries, synchronize data, or modify source books.

### DD-011: Treat images as a capability chain

**Date:** August 19, 2026 at 7:14 PM EDT

**Status:** Accepted

TermLeaf will normalize bounded raster and static SVG input, then render through
a positively detected Kitty, Sixel, or iTerm2 path. Unsupported terminals fall
back to true-color half-block cells and finally a useful caption. The reader
will attempt safely enabled formats but will not promise arbitrary media or emit
several graphics protocols blindly.

### DD-012: Store annotations beside reader state

**Date:** August 19, 2026 at 7:14 PM EDT

**Status:** Accepted

Bookmarks, colored highlights, and plain-text notes belong to versioned local
TermLeaf state and point to logical document ranges. They never rewrite TXT,
Markdown, or EPUB sources. Export, sharing, and synchronization remain outside
the first release.

### DD-013: Make Paper a responsive built-in theme

**Date:** August 19, 2026 at 7:14 PM EDT

**Status:** Accepted

Paper uses a warm ivory page, charcoal text, muted olive accents, restrained
sepia highlights, and a subtle centered boundary. It gives up margins or the
page boundary before content becomes cramped and has 256-color and monochrome
fallbacks. Its semantic color roles must retain 4.5:1 text contrast, and state
must never rely on color alone. It preserves source image colors and will not
fake texture, alter display hardware, or reduce contrast for decoration.

### DD-014: Make Rust quality rules part of the delivery contract

**Date:** August 19, 2026 at 11:30 PM EDT

**Status:** Accepted

`code_quality.md` defines required implementation, review, testing, security,
and dependency practices for TermLeaf. Exceptions must be narrow, justified,
tested, and recorded; schedule pressure is not an exception. The standard may
change when implementation evidence shows a better rule, but it must not be
silently bypassed.

### DD-015: Identify test evidence with stable case IDs

**Date:** August 19, 2026 at 11:49 PM EDT

**Status:** Accepted

`testcases.md` maps product requirements and risks to stable IDs, executable
profiles, fixtures, environments, and phase gates. IDs are never reused. A case
with an unresolved oracle remains Blocked behind a named decision instead of
passing several incompatible outcomes. Implementation and reports must identify
the exact selected IDs rather than relying on a broad statement that tests ran.

### DD-016: Keep UI implementation inside the six delivery phases

**Date:** August 20, 2026 at 12:05 AM EDT

**Status:** Accepted

`ui_mockups.md` defines responsive hierarchy, focus, modes, overlays,
accessibility, safety, and component guidance for the first-release screens.
The UI does not create a seventh phase: shell/state work belongs to Phase 0,
the TXT reader to Phase 1, structured semantics and images to Phase 2, dependable
local interactions to Phase 3, refinement evidence to Phase 4, and native
release evidence to Phase 5.

### DD-017: License TermLeaf under GPL-3.0-only

**Date:** August 20, 2026 at 12:15 AM EDT

**Status:** Accepted

TermLeaf uses the exact SPDX expression `GPL-3.0-only`. The Cargo package,
README notice, and repository license therefore grant GPL version 3 without the
optional "or any later version" election. This resolves the licensing choice
before Rust source distribution begins.

### DD-018: Bound Phase 0 terminal interruption and launch state

**Date:** August 20, 2026 at 12:44 AM EDT

**Status:** Accepted

Phase 0 supports raw-mode Ctrl-C on every target plus catchable external
`SIGINT` on POSIX. Windows console control events and other POSIX signals wait
for a safe native harness and persistence checkpoint policy. Kernel terminal
attributes return to their captured values. Write-only ANSI modes return to the
documented ordinary-shell baseline because no portable query can recover
arbitrary preexisting state on every promised terminal.

### {dd}: Fix the Phase 1 reader key map with a timer-free prefix

**Date:** August 21, 2026

**Status:** Accepted

Reading mode binds line movement to `j`/`k` and the arrow keys, page movement
to `PgUp`/`PgDn` and `Ctrl-B`/`Ctrl-F`, document start/end to `Home`/`End`
and `gg`/`G`, section start/end to `{`/`}`, mode selection to `p`/`c`, theme
selection to `t`, help to `F1`/`?`, back to `Esc`, and quit to `q`/`Ctrl-C`.
A lone `g` opens a prefix; a second `g` completes book-start, and any other
key cancels the prefix and is then mapped normally, so unrelated input is
never lost. The policy uses no timer, which keeps behavior deterministic in
tests. This is the initial `DEC-TEST-010` resolution; PTY keyboard evidence
and later text-entry modes may still adjust it before release.

### {dd}: Collapse status fields by documented priority with tick lifetime

**Date:** August 21, 2026

**Status:** Accepted

The status line renders title, chapter, logical location, dynamic page,
percentage, mode, UTC clock (`HH:MM`), and the help hint. When width runs
out, fields disappear whole in reverse priority: clock, dynamic page, title,
chapter, hint, location; percentage and mode never drop and truncation stays
character-boundary safe. Temporary messages replace lower-priority fields for
exactly eight delivered key events rather than wall-clock time. Percentage is
floored so forward movement is monotonic and resize never changes it. This is
the initial `DEC-TEST-011` resolution pending render review.

### {dd}: Bound plain-text input at 32 MiB with double size checks

**Date:** August 21, 2026

**Status:** Accepted

Plain-text books larger than 32 MiB (inclusive limit) are rejected with a
typed error before decoding. The limit is checked against file metadata first
and enforced again with a guarded `take(limit + 1)` read, so neither a lying
header nor a racing writer can force an unbounded allocation. Boundary tests
exercise below, exact, and above the limit. This partially resolves
`DEC-TEST-012` for TXT input; configuration, state, query, note, URL, recent,
annotation, and aggregate persisted-state limits remain open.
