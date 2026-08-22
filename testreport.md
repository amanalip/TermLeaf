# Test Report

**Last updated:** August 22, 2026 at 6:04 PM EDT

## Table of Contents

- [Purpose](#purpose)
- [Update Rules](#update-rules)
- [Local Gutenberg Corpus](#local-gutenberg-corpus)
- [Pending Commit](#pending-commit)
- [Commit Reports](#commit-reports)

## Purpose

This file records the evidence behind each TermLeaf commit. It distinguishes
checks that passed from checks that were skipped, unavailable, or not yet
applicable. It does not replace CI output, fuzzing artifacts, benchmark data, or
platform release records.

## Update Rules

Every commit must have one report entry that records:

- The commit subject and revision, using `This commit` when the report is part
  of the commit it describes or `Pending` before its subject is chosen.
- The behavior and risks exercised by the checks.
- The operating system and relevant tool versions.
- Each command or manual procedure and its result.
- Fixtures used, including provenance when they are not generated locally.
- Skipped or unavailable checks and the reason.
- Whether `cargo clean` ran after the complete local Rust validation cycle.
- Changed paths and their classified areas from `testcases.md`.
- Exact selected case IDs, profile manifests, commands, features, environments,
  fixture hashes, generated seeds, and external evidence links.
- For each Blocked case: owner, reason, compensating evidence, removal condition,
  and review date.

Never mark a check as passing when it was not run. Prepare the report with the
change and identify that atomic change as `This commit`; requiring its own hash
inside its contents would make the hash self-referential. Keep detailed CI logs
in CI rather than copying large logs into this file.

## Local Gutenberg Corpus

The following books were downloaded explicitly for development from Project
Gutenberg on August 19, 2026. They are public domain in the USA according to
their source pages. The files remain under ignored `downloads/gutenberg/` and
are not repository assets.

| Work | Gutenberg ID | Local formats | Purpose |
| --- | ---: | --- | --- |
| Alice's Adventures in Wonderland | 11 | EPUB 3 with images, UTF-8 TXT | EPUB 3 navigation, XHTML, illustrations, and text comparison |
| Frankenstein | 84 | Legacy EPUB without book illustrations, UTF-8 TXT | Longer chapter sequence and EPUB/TXT comparison |
| Pride and Prejudice | 1342 | Legacy EPUB without book illustrations, UTF-8 TXT | Larger prose document, chapter navigation, and sustained layout |

Download sources:

- `https://www.gutenberg.org/ebooks/11`
- `https://www.gutenberg.org/ebooks/84`
- `https://www.gutenberg.org/ebooks/1342`

The current local files occupy 2,499,711 bytes in total. Their SHA-256 values
identify the exact inputs used in this report:

| Local file | Bytes | SHA-256 |
| --- | ---: | --- |
| `alice-11.epub` | 189,231 | `6b79f2d23b804172816e81c463dbcea689593bbde63ef200d52b6c0da7ef629c` |
| `alice-11.txt` | 174,311 | `01b38ea4c710a84bc18d0bd41271a5a1a92b94e97b2812f4dece97d4a694725e` |
| `frankenstein-84.epub` | 356,351 | `2719565ac885c335df88f220b03a9c45b95dc4225193a8dc649f6493550c4332` |
| `frankenstein-84.txt` | 448,885 | `7810cd483cffcf2cc8a1d8f0d5807931e69d4f48cd14149b8c76f88af82fead3` |
| `pride-and-prejudice-1342.epub` | 558,547 | `462be7852d84412c6695851395144a97e9762d45bd3c41b9f356dc7ac047b8a9` |
| `pride-and-prejudice-1342.txt` | 772,386 | `74f2665d6e6925fc2c17dec644bec9e87df478a0f1836822125e8acbb3777806` |

Real books exercise ordinary document structure. Purpose-built fixtures must
still cover malformed archives, path traversal, compression bombs, excessive
resource use, invalid encoding, hostile SVG content, and other security limits.

## Pending Commit

### Decode bounded images

**Behavior and risks.** Lands the bounded raster-decode core of the Phase 2
image slice in a new `src/document/image.rs`: the locked initial image limits
table becomes `ImageLimits` policy (32 MiB compressed input, 8 MiB SVG/SVGZ
XML for the vector slice, 16,384 per-side dimensions, 64 million pixels, 256
MiB allocation), enforced strictly in order — the input byte gate rejects
before any parse, header-only dimension reads reject hostile geometry before
any pixel allocation, and a conservative per-decoder-family allocation
ceiling (RGBA8 4 B/px; PNG/TIFF 8; Radiance HDR 12; OpenEXR 16) keeps wide
intermediate buffers inside the envelope. Only then does decoding run,
normalizing to RGBA8 with first-frame-only animation. Format resolution is
extension-first with magic winning when present (`DEC-TEST-001` alignment),
so magic-less TGA decodes through its declared extension while a mislabeled
PNG still resolves by signature. Every rejection is typed with value versus
limit and recovery text. All fourteen enabled decoders round-trip generated
fixtures (DDS hand-crafted from the container spec as one DXT1 block); the
`image` crate joins at 0.25.x with exactly the locked format features.
Risks exercised: hostile geometry and byte bombs reject without allocating,
truncated/corrupt/foreign inputs fail typed without panicking.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI.

**Checks.**

| Check | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Pass (after regenerate) |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --locked` | Pass (149 library, 9 CLI, 15 document-I/O, 14 render, 6 property, 14 native PTY) |
| `cargo test --doc --locked` | Pass |
| `cargo deny check` | Pass with documented advisory exception |

**Dependency note.** The OpenEXR feature pulls `exr -> pulp -> paste`, which
carries RUSTSEC-2024-0436 (unmaintained, no known vulnerability). Ignored in
`deny.toml` with rationale and a revisit condition; licenses/bans/sources all
pass on the new graph.

**Fixtures.** No committed fixtures changed; every decoder fixture is
generated in-test (encoders where available, specification-crafted bytes for
DDS).

**Skipped coverage.** SVG/SVGZ vector decoding, half-block cell rendering,
worker queues/loading UI, protocol detection, and EPUB/Markdown `<img>`
ingestion stay with their owning slices; hosted environment rows remain to be
recorded until push. APNG first-frame evidence joins IMG-002 with its
integration slice.

**Selected case IDs.** `IMG-001`, `IMG-006`, `IMG-007` marked Implemented;
locations registered under `IMG-001`, `IMG-002`, `IMG-005`, `IMG-006`,
`IMG-007`, `IMG-012`, `SUP-009`.

**Cargo clean.** The complete local Rust validation cycle for this change
ends with `cargo clean`.

## Commit Reports

### Add table of contents navigation

**Commit subject:** `feat: add table of contents navigation`

**Revision:** `9d09e27`

**Recorded:** August 22, 2026 at 1:20 AM EDT

**Behavior and risks.** The contents overlay opens over any open book via
`o` or F2, seeds its cursor on the current reading section, scrolls long
section lists, labels untitled sections stably, jumps on Enter through the
validated section-start step with a confirmation message, returns exactly
via Escape, keeps help reachable with round-trip state, and never moves
the hidden anchor while open. ShowToc without a book stays inert. Focus
ownership maps to the existing TableOfContentsItem kind.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1.

**Checks.**

| Check | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Pass |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --locked` | Pass (140 library, 9 CLI, 15 document-I/O, 14 render, 6 property, 14 native PTY) |
| `cargo test --doc --locked` | Pass |

**Skipped coverage.** Hosted rows for this revision remain to be recorded;
a PTY journey for the overlay joins the Phase 2 gate evidence batch.

**Selected case IDs.** `NAV-009` locations extended (TOC half).

**Cargo clean.** The complete local Rust validation cycle for this change
ends with `cargo clean`, which removed 4,742 files (1.4 GiB).



### Complete semantic content and Markdown

**Commit subject:** `feat: complete semantic content and markdown`

**Revision:** `3ae3ae6`

**Recorded:** August 22, 2026 at 12:35 AM EDT

**Behavior and risks.** Completes the semantic content slices of Phase 2:
EPUB XHTML and Markdown now both produce the full reading-relevant
structure (headings, paragraphs, nested lists with depth and ordering,
quotes, verbatim code, separators, tables with per-cell ranges) plus
inline roles (emphasis, strong, inline code, links) carried as validated
decoration spans that never move canonical positions. Layout wraps
through role boundaries; list markers hang-indent at marker width with
per-depth numbering restarts; code renders one verbatim row per line;
tables align columns when natural width fits and linearize without
dropping cells otherwise. The Markdown path is strictly bounded (shared
32 MiB budget, metadata check then guarded read, exact inclusive
boundaries under injected limits), decodes strict UTF-8 after BOM strip,
and keeps raw HTML, scripts, and remote references completely inert
(nothing executes or resolves). Malformed constructs parse
deterministically. `.md`/`.markdown` detection is case-insensitive with
misleading content still failing typed after the gate. Renderer styles
each role distinctly by attribute so `NO_COLOR` sessions keep the
distinctions.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI.

**Checks.**

| Check | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Pass |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --locked` | Pass (137 library, 9 CLI, 15 document-I/O, 14 render, 6 property, 14 native PTY) |
| `cargo test --doc --locked` | Pass |
| `cargo deny check` | Pass (advisories, bans, licenses, sources incl. new pulldown-cmark graph) |
| `git diff --check` | Pass |

**Fixtures.** No committed fixtures changed; Markdown journeys use
generated temporary books.

**Skipped coverage.** Hosted rows for this revision remain open until
push. Render-profile wide/narrow evidence for the EPUB semantic fixture
(`EPUB-012`, `MD-003`) and image placement cases stay with their owning
slices.

**Changed paths.** `src/document/model.rs`, `src/document/xhtml.rs`,
`src/document/markdown.rs` (new), `src/document/epub.rs`,
`src/document/error.rs`, `src/document/mod.rs`, `src/layout/mod.rs`,
`src/layout/viewport.rs`, `src/app/state.rs`, `src/ui/reader.rs`,
`Cargo.toml`, `Cargo.lock`, `tests/cli.rs`, `tests/document_io.rs`,
`tests/case_registry.overrides.toml`, `tests/case_registry.toml`
(generated), tracker documents.

**Selected case IDs.** `MD-001`, `MD-002`, `MD-006`, `MD-008`, `MD-009`,
`MD-012`, `LAY-009`, `LAY-010` Implemented; `EPUB-011` and `CLI-007`
extended; `MD-003`, `MD-004`, `MD-005`, `MD-007`, `MD-010`, `MD-011`
remain Planned with named remaining halves.

**Cargo clean.** The complete local Rust validation cycle for this change
ends with `cargo clean`, which removed 4,614 files (1.4 GiB).



### Harden structured ingestion boundaries

**Commit subject:** `feat: harden structured ingestion boundaries`

**Revision:** `437ea4c`

**Recorded:** August 21, 2026 at 10:10 PM EDT

**Behavior and risks.** Closes the two P0 security slices named by the
tracker after manifest fallbacks. Structural bounding: XHTML chapters now
carry an explicit markup-node budget aligned with the EPUB limits table;
the scan counts `<` openings on raw bytes before the HTML5 tree builder
allocates anything, so hostile or corrupt chapters stop with the new typed
`ChapterTooComplex` error naming the book path, archive member, observed
count, and limit instead of consuming parser memory. Boundary Method
evidence holds exactly at the policy edge (one million openings converts,
one million and one rejects) and under injected smaller budgets, including
proof that the default constant matches the documented table. Byte
stability (`DD-027` follow-through): ingestion now exposes the two stages
as a public `EpubSnapshot`, whose `open` reads and preflights the source
once and closes the file handle, and whose `build` resolves package
semantics over only the inspected immutable bytes. Integration journeys
overwrite the source with a second complete book, truncate it to zero,
append garbage, rename it away, delete it outright, and (on Unix) swap the
path for a symlink to a decoy edition between the two stages; every journey
still returns the originally inspected title and passage, proving no step
re-opens or re-reads an unchecked path. Scope note: the structural budget
applies where TermLeaf parses directly (chapters); control documents keep
their 16 MiB actual-byte preflight bound plus `rbook`'s non-recursive pull
parser as compensating controls while `SEC-009` remains open for direct
control-document gates. Real-book behavior is unchanged: all previously
passing suites, including the Gutenberg smoke checks in the earlier report,
still pass without fixture changes.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI.

**Checks.**

| Check | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Pass |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --locked` | Pass (116 library, 9 CLI, 13 document-I/O, 14 render, 6 property, 14 native PTY) |
| `cargo test --doc --locked` | Pass |
| `cargo deny check` | Pass (advisories, bans, licenses, sources) |
| `git diff --check` | Pass |

**Fixtures.** Committed FX-EPUB2/FX-EPUB3 unchanged; temporary books are
generated per test and removed with the test process. No new fixtures.

**Skipped coverage.** Hosted CI rows for this revision remain open until
push; Windows symlink-swap runs only on Unix rows because unprivileged
symlink creation differs there, while rename/delete halves run everywhere.

**Changed paths.** `src/document/xhtml.rs`, `src/document/epub.rs`,
`src/document/error.rs`, `src/document/mod.rs`,
`tests/document_io.rs`, `tests/case_registry.overrides.toml`,
`tests/case_registry.toml` (generated), tracker documents.

**Selected case IDs.** `EPUB-005` (extended locations), `EPUB-010`
(Implemented), `EPUB-016` (Implemented); `SEC-009` remains Planned with
chapter-side progress noted in the tracker.

**Cargo clean.** The complete local Rust validation cycle for this change
ends with `cargo clean`, which removed 4,052 files (1.2 GiB).

### Start the structured book ingestion

**Commit subject:** `feat: start structured book ingestion`

**Revision:** `bed99a8`

**Recorded:** August 21, 2026 at 8:19 PM EDT

**Behavior and risks.** Opens Phase 2 with the bounded ZIP preflight and
`rbook` semantics named by the tracker. New behavior exercised: the archive
layer (`DD-008`) rejects absolute/UNC/parent-escaping/NUL/colon member names
and duplicate canonical keys before semantic parsing; symlink entries,
encrypted flags, unsupported compression methods, overlapping compressed
regions, truncated/corrupt central directories, and CRC failures each fail
with one typed policy error; inclusive boundaries hold exactly at the
256 MiB compressed size, 10,000 members, 512 MiB advertised expansion,
16 MiB control resources (container/OPF/NCX), 32 MiB XHTML chapters, and
the 100:1 ratio rule above the 64 KiB small-file exception including
zero-byte handling and saturating arithmetic; control and chapter resources
prove their actual decompressed bytes against declared sizes while images
and fonts stay lazy; `.epub` joins the extension table case-insensitively
(`DEC-TEST-001`) with misleading content still failing after the gate.
On top of the boundary layer: minimal EPUB 2 (NCX) and EPUB 3 (nav)
fixtures open with correct titles, linear spine order, TOC-derived chapter
titles, and heading/paragraph structure; nonlinear spine items stay outside
the reading order; missing titles fall back to the file stem; encrypted and
fixed-layout books receive their specific messages; malformed XHTML
recovers readably through the HTML5 tree builder with scripts/styles
dropped, entities decoded, `<br>` breaks preserved, and hostile nesting
depth bounded deterministically; multi-section documents tile the canonical
text across sections and layout rows now carry section-qualified block
ownership. A real-book smoke check parsed all three local Gutenberg EPUBs
(Alice: 14 sections; Frankenstein: 30; Pride and Prejudice: 15) with
correct metadata titles.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI.

**Commands and results.**

| Command | Result |
| --- | --- |
| `python3 tools/make_epub_fixtures.py` | Passed: deterministic FX-EPUB2/FX-EPUB3 written with recorded SHA-256 values |
| `cargo run --example real_books_smoke` (scratch, removed) | Passed: three Gutenberg EPUBs parsed with expected titles and section counts |
| `python3 tools/case_registry.py generate` | Passed: manifests regenerated for new locations |
| `python3 tools/case_registry.py check` | Passed: bidirectional validation clean after overrides |
| `cargo fmt --check` | Passed: no diff |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed: no warnings |
| `cargo test --locked` | Passed: 114 library, 9 CLI, 11 document-I/O, 14 render, 6 property, and 14 native PTY tests, 0 failed |
| `cargo test --doc --locked` | Passed: 0 doctests |
| `cargo deny check` | Passed: advisories, bans, licenses (added ISC for scraper), sources |

**Fixtures.** Committed `FX-EPUB2` and `FX-EPUB3`, generated deterministically
by `tools/make_epub_fixtures.py`
(FX-EPUB2 sha256 `fa15e4b3867ff80784c214ce73ff3467077569cffc0915a078362ebdc9f5a44c`,
FX-EPUB3 sha256 `5b0864436c894203ede28cc98246ae77ff53e6b9eab9dfa4216118afd33c7658`).
Synthetic in-test archives cover hostile names, crafted headers, overlaps,
encryption flags, truncation, corruption, and CRC lies via a byte-level
builder plus the `zip` writer. Real-book smoke used the previously recorded
local Gutenberg corpus only.

**Selected exact IDs.** Implemented this change: `SEC-001`, `SEC-002`,
`SEC-003`, `SEC-004`, `SEC-005`, `SEC-006`, `SEC-007`, `SEC-008`, `SEC-010`,
`SEC-011`, `EPUB-001`, `EPUB-002`, `EPUB-003` (linear/nonlinear spine half),
`EPUB-004`, `EPUB-005`, `EPUB-006`, `EPUB-007`, `EPUB-009`, `EPUB-011`
(headings/paragraphs/breaks half), `EPUB-014`. Evidence extended for
already-Implemented `MODEL-001` (multi-section tiling) and `CLI-007`
(EPUB extension acceptance plus post-gate content validation).

**Skipped or unavailable, with forward ownership.**

| Check | Reason and removal condition |
| --- | --- |
| `EPUB-003` fallback-manifest half | `DEC-TEST-015` fixture work lands with the next EPUB slice; the linear/nonlinear half passes today. |
| `EPUB-008`, `EPUB-010`, `EPUB-012`, `EPUB-013`, `EPUB-015`, `EPUB-016` | Links, byte-stability instrumentation, wide/narrow semantic render journeys, image placement, and filesystem side-effect audits arrive with later Phase 2 slices. |
| `EPUB-009` full remote/network matrix | Script/style inertness passes; DTD/entity and remote-media variants join the security-profile slice with FUZZ targets. |
| `SEC-009` XML depth/nodes per control document | Requires the XML parser boundary slice that precedes OPF/NCX semantic checks in-repo; archive-level bounding is in place. |
| `MD-*`, `IMG-*`, `CON-*`, `FUZZ-*`, hosted rows for this revision | Owning features or CI execution land later in Phase 2. |

**Changed paths and classified areas.** `Cargo.toml`/`Cargo.lock`
(dependencies: `zip` 8.6 shared with rbook, `rbook` 0.7.10, `scraper`
0.27, `ego-tree`; dev-only `zip` writer access),
`src/document/archive.rs` (new bounded preflight), `src/document/epub.rs`
(new package semantics), `src/document/xhtml.rs` (new tolerant converter),
`src/document/model.rs` (multi-section model, headings),
`src/document/error.rs` (typed EPUB errors, `.epub` detection),
`src/document/mod.rs` (unified loader dispatch), `src/app/state.rs`
(load dispatch), `src/layout/mod.rs` (section-qualified rows),
`tests/document_io.rs`, `tests/cli.rs` (integration evidence),
`tests/case_registry.overrides.toml` plus regenerated manifests,
`tests/fixtures.toml`, `tools/make_epub_fixtures.py` (fixtures).

**Blocked cases.** Unchanged from the Phase 1 gate record.

**Cleanup.** `cargo clean` ran after this complete local Rust validation
cycle and removed 7,195 files (2.0 GiB).

### Complete the Phase 1 gate

**Commit subject:** `test: close the Phase 1 gate evidence`

**Revision:** `344f9fb`, completed by `db27a0f` (ConPTY paste-journey scoping)

**Recorded:** August 21, 2026 at 7:12 PM EDT

**Behavior and risks.** Closes every Phase 1-closeable gap in the frozen
`phase-gate-1` manifest. New behavior exercised: the complete reader key
matrix inside native PTYs on every required environment row (Up, Down,
PageUp/PageDown, Ctrl-B/Ctrl-F, Home/End, F1, Escape versus Alt chords,
Ctrl-C); bracketed paste inertness including multiline, control-containing,
and 64 KiB oversized payloads in both Phase 1 modes; repeated resizes through
a tiny transient size and back to the same logical anchor; focus, mouse,
resize, release, and paste events staying inert at the event-filter boundary
with prefix state preserved; deterministic seeded properties for row width
bounds, grapheme integrity, anchor survival across resize sequences, page
progression and its exact inverse semantics, and action-sequence state
validity; extension-first format detection accepting case-insensitive `.txt`
and rejecting every other or missing extension pre-terminal while still
strictly decoding `.txt` content (misleading pairs covered both ways);
caret-notation escaping of control bytes in failing-path diagnostics;
read-only and immutable source guarantees; right-to-left samples staying
bounded at five widths; locale variants rendering identical Unicode;
ambiguous-width characters pinned to the narrow measurement; Paper collapse
order asserted cell-by-cell from wide canvas to boundary removal; a Paper
capability matrix over three color modes by five viewports by reader and
help states; true-color role values verified at exact cells; theme switches
at mid-passage preserving the anchor across all five themes; status field
collapse order proven by first-drop widths with deterministic message
lifetime restore; redraw stability with single-field changes; help reachable
from Recent books, Reader, Themes, and over itself, returning through the
overlay stack to the exact passage.

One navigation defect was found and fixed by the new property suite:
previous page was not the inverse of next page when blank spacer rows or
end clamping intervened. The backward step now searches for the smallest
content row whose unclamped forward step lands exactly on the current page,
restoring prior anchors exactly whenever the forward hop fit inside the
document, with a bounded fallback hop otherwise (`src/reader.rs`).

The hosted Windows rows surfaced a ConPTY transport limitation during this
gate run (GitHub Actions run `32535423048`): ConPTY's input pipeline parses
the bracketed-paste markers themselves and forwards the inner bytes as
ordinary keystrokes, so a programmatic paste cannot arrive as a Paste event
through that transport. The marker-based journey is therefore scoped to
platforms whose PTY layer preserves it (`#[cfg(not(windows))]`), the
platform-independent Paste inertness stays proved by the term_007 event
filter on every platform, and real Windows terminals belong to the release
matrix per DD-026.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI. PTY cases run under
the hermetic harness with `LANG`/`LC_ALL=C.UTF-8`; the locale case varies
both to `C` and `en_US.UTF-8` deliberately.

**Commands and results.**

| Command | Result |
| --- | --- |
| `python3 tools/case_registry.py generate` | Passed: manifests regenerated; gate membership unchanged except catalog text for resolved DEC-TEST-001 |
| `python3 tools/case_registry.py check` | Passed: bidirectional validation clean after overrides |
| `cargo fmt --check` | Passed: no diff |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed: no warnings |
| `cargo test --locked` | Passed: 97 library, 8 CLI, 4 document-I/O, 14 render, 6 property, and 14 native PTY tests, 0 failed |
| `cargo test --doc --locked` | Passed: 0 doctests |
| `cargo deny check` | see below |

**Fixtures.** Synthetic only: generated journey/flow/escape/paste/resize/
locale books (60 or 20 numbered paragraphs), generated property documents
from a fixed-seed xorshift generator over an ASCII/CJK/combining/ZWJ/flag/
skin-tone/tab alphabet, sparse oversized `.txt`, read-only temp files, and
Arabic/Hebrew/mixed-direction samples built from Unicode escapes. No
downloaded or repository assets were involved.

**Selected exact IDs.** Implemented this change: `CLI-003`, `CLI-007`,
`TXT-010`, `MODEL-002`, `LAY-007`, `LAY-011`, `LAY-013`, `LAY-014`,
`LAY-015`, `NAV-009`, `NAV-010`, `THEME-001`, `THEME-003`, `THEME-004`,
`THEME-006`, `THEME-009`, `RENDER-002`, `RENDER-003`, `RENDER-004`,
`STATUS-001`, `STATUS-006`, `HELP-001`, `ERR-001`, `ERR-003`, `PROP-001`,
`PROP-002`, `PROP-003`, `PROP-004`, `PROP-010`, `KEY-001`, `KEY-002`,
`KEY-003`, `KEY-006`, `KEY-007`, `TERM-006`, `TERM-007`. Evidence extended
for already-Implemented `LAY-001`..`LAY-008`, `NAV-001`..`NAV-008`,
`NAV-011`..`NAV-014`, `STATUS-002`..`STATUS-005`, `THEME-002`, `RENDER-001`,
which re-ran inside the same suites.

**Skipped or unavailable, with forward ownership (DD-026).**

| Check | Reason and removal condition |
| --- | --- |
| `KEY-005` | Search/note text entry does not exist until Phase 3; execute then per `manual_procedures.md`. Removal condition: Phase 3 KEY procedures recorded. |
| `NAV-008` P0 | Search/note-editing modes do not exist until Phase 3; the inertness principle is enforced today at the event filter (`term_007`). Owned forward to Phase 3; removal when SEARCH/UI text-entry cases land. |
| `NAV-009` TOC/annotation halves | Those views arrive in Phases 2-3; the help half passes now. |
| `NAV-013` chapter/TOC/search/bookmark/highlight/note jumps | Owning features arrive in Phases 2-3; line/page/start/end jumps pass in both modes today. |
| `LAY-009`, `LAY-010` code/table halves | Markdown/EPUB semantics land in Phase 2; long-line wrap policy is exercised by TXT cases today. |
| `THEME-005` terminal-default visual checks | Automated TerminalDefault matrix passes; real-terminal visual review belongs to the Deferred environment rows. |
| `THEME-007` selection/search/link/warning colors | Selection/search/link arrive in later phases; accent focus plus non-color cues are rendered today. |
| `THEME-008` images | Images arrive in Phase 2. |
| `STATUS-007` failed-save state | Persistence arrives in Phase 3; layout-derived page versus logical location is covered by LAY-006/LAY-007 today. |
| `ERR-003` note-content half | Notes arrive in Phase 3; control-byte escaping of diagnostics passes now. |
| `PROP-005`..`PROP-009` | Search/state/archive/image/concurrency features own these properties in their phases. |
| `KEY-001`/`KEY-002`/`KEY-006` human-terminal halves, `LAY-013` font half, `LAY-014` observation | Require Deferred GUI environment rows; procedures written in `manual_procedures.md`; owned by the release native matrix. |
| Provisional PERF budgets | Benchmarks are owned by Phase 4 per the registry; budgets stay provisional pending representative-hardware recording, so no gate-1 benchmark membership exists. |
| Hosted rows for `29a730a` (Phase 1 implementation) | GitHub Actions run `32529070577`: all eight jobs passed (Rust checks and native PTY on ubuntu-24.04, macos-15, windows-2025; dependency policy; Rust 1.88). |
| Hosted rows for this gate revision | Run `32535423048` (`344f9fb`) failed only the two Windows jobs on the ConPTY paste finding above; run `32535725291` (`db27a0f`) passed all eight jobs: Rust checks and Native PTY on ubuntu-24.04/macos-15/windows-2025, Dependency policy, Rust 1.88. Required environment rows ENV-LINUX-PTY, ENV-MAC-PTY, and ENV-WIN-PTY are thereby evidenced at revision `db27a0f`. |

**Changed paths and classified areas.** `Cargo.toml`/`Cargo.lock`
(dependency, dev-only `unicode-segmentation`), `src/document/*` (plain-text
ingestion, document model, typed errors), `src/app/*` (state, key map,
theme-overlay help), `src/process.rs` (process boundary diagnostics),
`src/reader.rs` (navigation policy fix), `src/layout/width.rs`
(layout/Unicode), `src/terminal.rs` (event filter), `tests/cli.rs`,
`tests/document_io.rs`, `tests/render.rs`, `tests/properties.rs`,
`tests/pty_native.rs` (integration targets), `tools/case_registry.py`
(registry tooling), `tests/case_registry.overrides.toml` plus regenerated
manifests (test fixtures), `testcases.md` (resolved decision),
`manual_procedures.md` (manual layer).

**Blocked cases.** Unchanged: `TERM-011`/`TERM-012` hosted rows remain owned
per `DD-018`; hosted evidence for those lifecycle rows was produced again by
CI runs recorded against earlier revisions and will be refreshed for this
revision on push.

**Cleanup.** `cargo clean` ran after this complete local Rust validation
cycle and removed 3,537 files (819.3 MiB).

### Complete the plain-text reading loop

**Commit subject:** `feat: complete the plain-text reading loop`

**Revision:** This commit

**Recorded:** August 21, 2026 at 5:14 PM EDT

**Behavior and risks.** Completes the remaining Phase 1 implementation items.
New behavior exercised: configuration loading from the platform config
directory with default < config.toml < `--theme` precedence; tolerant
fallback for missing, unreadable, wrong-typed, and malformed TOML; every
theme slug round-tripping through config without the file ever being
rewritten; output color-capability detection (`COLORTERM`, `TERM`) with an
exact nearest-xterm-256 mapping that preserves modifiers, keeps palette
entries as fixed points, and degrades unknown terminals to terminal-default;
native PTY render journeys covering Down/PageDown/Home/`G`/help/Escape/`gg`
with restoration checks; startup-theme selection observable over PTY through
the selection cursor and applied marker; sparse-file evidence that books
above the byte limit fail before terminal setup with the full typed message.

The journeys exposed three real defects that this change fixes:

1. Uppercase bindings (`G`, `?`) never fired on real terminals because
   crossterm reports capitals as `Char + SHIFT` while the registry matched
   bare characters. Character keys now drop SHIFT before matching.
2. The event loop built a fresh mapper per event, so the timer-free `gg`
   prefix could never complete across reads. The loop now owns one
   persistent [`KeyMapper`] for the session.
3. Reader navigation and mode actions leaked into overlay views such as
   help, silently moving the hidden anchor. They are now inert outside the
   Reader view, and temporary messages render in the non-reader status line
   so confirmations are visible everywhere.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1,
cargo-deny 0.20.2; MSRV target unchanged at 1.88 in CI.

**Commands and results.**

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Passed: no diff |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed: no warnings |
| `cargo test --locked` | Passed: 88 library, 6 CLI, and 9 native PTY tests, 0 failed |
| `cargo test --doc --locked` | Passed: 0 doctests |
| `python3 tools/case_registry.py generate` | Passed: manifests regenerated; frozen gate membership unchanged |
| `python3 tools/case_registry.py check` | Passed: bidirectional validation clean |
| `cargo deny check` | Passed after allowing `BSD-3-Clause` (`encoding_rs` bundled WHATWG data) and `MPL-2.0` (`option-ext` via `directories`) |

**Fixtures.** Synthetic only: generated journey book (60 numbered
paragraphs), generated `config.toml`, and a sparse oversized `.txt` created
with `set_len`; no downloaded or repository assets involved.

**Skipped or unavailable.**

| Check | Reason |
| --- | --- |
| Hosted platform/MSRV/PTY jobs | Require pushed CI runs; recorded separately when they execute. New dependencies (`serde`, `toml` 1.1.x, `directories` 6.x) are Core Stack selections from `project_plan.md`. |
| Phase-gate exit run | Gate evidence requires hosted environment rows plus the manual procedures (`KEY-001` full matrix, `KEY-005`, `KEY-006`, `LAY-013`, `LAY-014`); not claimed by this commit. |
| Render-profile snapshot review | `pr-render` remains Planned; Paper viewport collapse review stays open. |

**Changed paths and classified areas.** `Cargo.toml`/`Cargo.lock`
(dependency), `src/persistence/*` (configuration), `src/cli.rs`
(command-line surface), `src/process.rs` (process boundary), `src/app/*`
(state, key map, view scoping), `src/ui/theme.rs` and `src/ui/mod.rs`
(theme or UI), `src/terminal.rs` (event loop), `tests/cli.rs`,
`tests/pty_native.rs` (integration), `tests/case_registry.overrides.toml`
plus regenerated manifests (test fixtures).

**Selected exact IDs.** Implemented this change: `CFG-001`, `CFG-002`,
`CFG-003`, `TXT-008`, `THEME-002`. Location-only evidence added or extended
for: `KEY-001`, `KEY-003`, `THEME-005`. All remaining `phase-gate-1` IDs
stay Planned pending their declared layers, profiles, or hosted rows.

**Blocked cases.** Unchanged from the Phase 0 report: `TERM-011`/`TERM-012`
hosted-environment rows remain owned by their target phases per `DD-018`.

**Cleanup.** `cargo clean` ran after this complete local Rust validation
cycle and removed 2,327 files (596.5 MiB).

### Start the plain-text reading loop

**Commit subject:** `feat: start the plain-text reading loop`

**Revision:** `c65061d`

**Recorded:** August 21, 2026 at 11:08 AM EDT

**Behavior and risks.** Starts Phase 1, the plain-text reading loop, on top of
the Phase 0 foundation. New risks exercised: bounded file reading before
allocation, strict UTF-8 versus BOM-marked UTF-16 decoding, unmarked-UTF-16
rejection, newline normalization, paragraph/blank-line preservation,
grapheme-safe cell-width wrapping with source-range mapping, anchor-preserving
navigation across resize, the hybrid key map with a timer-free `gg` prefix
policy, semantic-role themes including Paper contrast and `NO_COLOR`, the
priority-collapsing status line with input-driven message lifetime, theme
selection, help listing every registered binding, and the below-minimum
suspension/recovery state.

**Environment.** Arch Linux (kernel 6.x, x86-64), rustc/cargo 1.97.1, MSRV
target unchanged at 1.88 in CI; `cargo-deny` not installed locally.

**Commands and results.**

| Command | Result |
| --- | --- |
| `cargo fmt --check` | Passed: no diff |
| `cargo clippy --all-targets --all-features -- -D warnings` | Passed: no warnings |
| `cargo test --locked` | Passed: 79 lib + 4 CLI + 7 native PTY tests, 0 failed |
| `cargo test --doc --locked` | Passed: 0 doctests |
| `python3 tools/case_registry.py generate` | Passed: manifests regenerated; frozen gate membership unchanged |
| `python3 tools/case_registry.py check` | Passed: bidirectional validation clean |
| Manual PTY smoke (Python `pty`) | Passed: Alice fixture and the local Pride and Prejudice TXT open, render Paper chrome plus status, jump to end (`G`), toggle themes (`t`+Enter), quit cleanly with exit code 0 |

**Fixtures.** Synthetic in-test fixtures only for unit coverage; manual smoke
used `downloads/gutenberg/pride-and-prejudice-1342.txt` (SHA-256 above) and a
generated `/tmp` Alice excerpt. No repository assets changed.

**Skipped or unavailable.**

| Check | Reason |
| --- | --- |
| `cargo deny check` | `cargo-deny` is not installed on this machine; CI runs it. New dependencies (`encoding_rs`, `unicode-segmentation`, `unicode-width`, `unicode-linebreak`, `thiserror`) are exactly the Core Stack selections from `project_plan.md`. |
| Hosted platform/MSRV jobs | Require pushed CI runs; recorded separately when they execute. |
| Phase-gate exit run | Phase 1 is in progress; gate evidence is not claimed. |

**Changed paths and classified areas.** `Cargo.toml`/`Cargo.lock`
(dependency), `src/document/*` (plain-text ingestion, document model),
`src/layout/*` (layout/Unicode), `src/reader.rs` (navigation/reading mode),
`src/app/action.rs` and `src/app/state.rs` (navigation or reading mode,
theme/UI state), `src/ui/*` (theme or UI), `src/clock.rs` (status support),
`src/lib.rs`, `tests/case_registry.overrides.toml` plus regenerated manifests
(tests/fixtures).

**Selected exact IDs.** Implemented this change: `TXT-001`–`TXT-006`,
`MODEL-004`, `LAY-003`, `LAY-004`, `NAV-003`, `NAV-004`, `NAV-007`,
`NAV-012`, `NAV-014`, `STATUS-003`, `STATUS-005`, `THEME-010`. Location-only
evidence added for: `KEY-002`, `KEY-003`, `TXT-007`–`TXT-009`, `MODEL-001`,
`MODEL-003`, `LAY-001`, `LAY-002`, `LAY-005`, `LAY-006`, `LAY-008`,
`LAY-012`, `NAV-001`, `NAV-002`, `NAV-006`, `NAV-011`, `RENDER-001`,
`STATUS-002`, `STATUS-004`, `THEME-001`, `THEME-002`, `THEME-005`,
`HELP-003`, and refreshed `APP-003`. All remaining `phase-gate-1` IDs stay
Planned or Blocked pending their declared layers.

**Blocked cases.** Unchanged from the Phase 0 report: `TERM-011`/`TERM-012`
hosted-environment rows remain owned by their target phases per `DD-018`.

**Cleanup.** `cargo clean` ran after this complete local Rust validation
cycle and removed 3,724 files (780.3 MiB).

### Distinguish ConPTY host controls

**Commit subject:** `test: distinguish ConPTY host controls`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:49 AM EDT

Scope:

- Use diagnostics from GitHub Actions run `32336883054` to identify ConPTY's
  process envelope separately from TermLeaf output.
- Retain strict Unix byte-level validation and use exact application lifecycle
  sequences for the equivalent Windows pre-terminal assertion.

Checks:

| Command or procedure | Result |
| --- | --- |
| GitHub Actions run `32336883054` | Diagnosed: ConPTY emitted `?9001`, focus, title, and cursor-show controls; all other Windows assertions passed |
| `cargo fmt --check` | Passed after formatting the sequence assertion |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed locally |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `cargo deny check` | Passed: advisories, bans, licenses, and sources |
| `python3 tools/case_registry.py check` | Passed |
| `cargo clean` | Passed: removed 1,996 files and 432.1 MiB |

### Filter echoed ConPTY negotiation

**Commit subject:** `test: filter echoed ConPTY handshake`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:45 AM EDT

Scope:

- Diagnose GitHub Actions run `32336582895`: Windows still captured the echoed
  cursor-position report after removing the earlier ConPTY query.
- Remove both host-negotiation sequences before evaluating application output
  or the VT100 model.
- Keep the CLI help assertion focused on stable syntax and argument content.

Checks:

| Command or procedure | Result |
| --- | --- |
| GitHub Actions run `32336582895` | Diagnosed: all application behavior passed except echoed host negotiation and an over-specific usage assertion |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed locally |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `cargo deny check` | Passed: advisories, bans, licenses, and sources |
| `python3 tools/case_registry.py check` | Passed |
| `cargo clean` | Passed: removed 1,996 files and 432.1 MiB |

### Stabilize Windows CLI assertions

**Commit subject:** `test: isolate ConPTY negotiation output`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:40 AM EDT

Scope:

- Diagnose the two remaining Windows failures from GitHub Actions run
  `32336321311` after five of six native PTY cases passed.
- Fix Clap's Windows `.exe` display-name variation at the command definition.
- Reset captured terminal output after the ConPTY startup handshake so assertions
  inspect only bytes emitted by the application.

Checks:

| Command or procedure | Result |
| --- | --- |
| GitHub Actions run `32336321311` | Diagnosed: Windows help name varied; ConPTY's host query reached the application-output assertion |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed locally |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `cargo deny check` | Passed: advisories, bans, licenses, and sources |
| `python3 tools/case_registry.py check` | Passed |
| `cargo clean` | Passed: removed 1,996 files and 432.1 MiB |

### Complete the ConPTY harness fix

**Commit subject:** `fix: answer ConPTY startup query`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:35 AM EDT

Scope:

- Diagnose the Windows PTY startup deadlock from the newly captured `ESC[6n`
  output in all six cases.
- Respond with a valid cursor-position report so ConPTY resumes child startup.
- Replace the non-Unix unit terminal-state alias that Windows Clippy rejected
  with an explicit marker type.

Checks:

| Command or procedure | Result |
| --- | --- |
| GitHub Actions run `32335908510` | Diagnosed: ConPTY emitted `ESC[6n`; Windows Clippy rejected unit-state bindings and comparison |
| ConPTY protocol audit | Passed: cursor-position request is answered once with `ESC[1;1R` through the PTY writer |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed locally |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `cargo deny check` | Passed: advisories, bans, licenses, and sources |
| `python3 tools/case_registry.py check` | Passed |
| `cargo clean` | Passed: removed 2,128 files and 442.5 MiB |

### Fix Windows CI regressions

**Commit subject:** `fix: restore Windows CI compatibility`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:26 AM EDT

Scope:

- Reproduce the Windows warnings-as-errors failure by inspecting its target-
  conditional compilation path.
- Gate the Unix-only `bail` import with the same `cfg(unix)` condition as all
  of its call sites.
- Preserve `SystemRoot`, `SystemDrive`, `WINDIR`, `ComSpec`, `PATH`, `PATHEXT`,
  and `OS` in the otherwise isolated Windows ConPTY child environment.
- Create redirected temp/state directories before spawn and retain child output
  in timeout diagnostics.

Checks:

| Command or procedure | Result |
| --- | --- |
| `cargo fmt --check` | Passed after formatting the ConPTY environment allowlist |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed locally; the exact Windows target awaits CI rerun |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `cargo deny check` | Passed: advisories, bans, licenses, and sources |
| Unix-only import audit | Passed: the `bail` import and call sites use matching conditions |
| `cargo clean` | Passed: removed 2,000 files and 433.3 MiB |

### Complete Phase 0 implementation

**Commit subject:** `feat: complete Phase 0 implementation`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:02 AM EDT

Scope:

- Complete first-release view/focus identities and foundation action-state
  invariants without implementing later screen behavior early.
- Retain one read-only source handle and reject unreadable input before terminal
  initialization.
- Resolve `DEC-TEST-013` with a bounded interrupt family and document the
  supported shell launch baseline for write-only ANSI modes.
- Generalize the native PTY target for Unix and Windows/ConPTY, including raw
  Ctrl-C, external `SIGINT`, and captured native terminal attributes.
- Add exact `APP-001` through `APP-004` cases and remove broad later-phase cases
  from the Phase 0 gate.

Selection:

- Changed areas: application state, CLI/startup, terminal lifecycle, process
  interrupts, native PTY tests, dependencies, test governance, and documentation.
- Selected case IDs: all exact `phase-gate-0` IDs in
  `tests/phase_gates.toml`, including `APP-001` through `APP-004`, `CLI-006`,
  `TERM-011`, and `TERM-012`.
- Profiles run: registry freshness, `pr-core`, local native PTY, and dependency
  policy.
- Fixtures: synthetic temporary files and case-owned PTYs only; no book corpus
  fixture was opened.
- Environment: Linux 7.1.8-1-cachyos x86-64, Rust 1.97.1, Python 3.14.7,
  cargo-deny 0.20.2, and `TERM=xterm-256color` inside `80x24` PTYs.

Checks:

| Command or procedure | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Passed: 336 unique IDs and no Phase 0 case lacks implementation evidence |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed after interrupt and Unix signal test dependency additions |
| `git diff --check` | Passed |
| `cargo clean` | Passed: removed 2,595 files and 485.7 MiB |

Unavailable environment evidence:

- Rust 1.88, Ubuntu 24.04, macOS 15, and Windows Server 2025/ConPTY jobs cannot
  run in this local environment. Their fixed CI definitions remain the formal
  evidence path and do not leave a Phase 0 implementation case incomplete.

### Continue the Rust foundation

**Commit subject:** `test: add Phase 0 manifests and PTY harness`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:44 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; rustc/cargo 1.97.1;
Python 3.14.7; cargo-deny 0.20.2; `TERM=xterm-256color` in PTY cases

Scope:

- Materialize and validate all 332 catalog IDs, exact profile assignments,
  fixture/environment references, test locations, and cumulative gate members.
- Isolate CLI and PTY processes with case-owned paths, minimal environments,
  fixed `80x24` PTYs, VT100 parsing, kernel terminal-state comparisons, and
  10-second deadlines with kill/reap cleanup.
- Exercise normal exit, raw-mode Ctrl-C, pre-terminal failure, active handled
  error, and recoverable panic restoration through native Linux PTYs.
- Delay handled-error and panic diagnostics until terminal cleanup completes.

Selection:

- Changed paths: Cargo dependencies/lockfile, process and terminal boundaries,
  CLI/PTY tests, generated test manifests and validator, CI, test catalog,
  README, implementation/commit trackers, and test report.
- Classified areas: CLI/startup, terminal lifecycle, tests/fixtures, CI,
  dependency graph, documentation, and test governance.
- Selected case IDs: `QG-001` through `QG-005`, `QG-007` through `QG-014`,
  `CLI-001`, `CLI-002`, `CLI-004`, `CLI-005`, `CLI-010`, `TERM-001` through
  `TERM-005`, `TERM-008`, `KEY-004`, `HELP-002`, `HELP-003`, `ERR-002`,
  `PROP-010`, `SUP-001` through `SUP-004`, and `SUP-006` through `SUP-008`.
- Profiles run: registry freshness, `pr-core`, Linux `native-pty`, and dependency
  policy. Planned render, security-target, scheduled, weekly, and release target
  commands did not run because their feature implementations do not exist yet.
- Fixtures: no book fixture was opened. `tests/fixtures.toml` records planned
  assets and the existing ignored Gutenberg provenance/hashes.
- Environment: local `ENV-LINUX-PTY` equivalent on CachyOS rather than the
  required Ubuntu 24.04 CI row; no macOS or Windows environment was claimed.

Checks:

| Command or procedure | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Passed: 332 unique IDs, no unknown/orphan locations, profiles and six cumulative gates agree |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 15 library tests, 3 CLI tests, and 5 Linux PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed: advisories, bans, licenses, and sources after dev-dependency additions |
| `git diff --check` | Passed |
| `cargo clean` | Passed: removed 1,886 files and 409.9 MiB |

Blocked or unavailable Phase 0 evidence:

- `TERM-011` remains Blocked, owned by Phase 0. `DEC-TEST-013` has not selected
  supported process signals. Raw-mode Ctrl-C key input is compensating evidence;
  removal requires a resolved signal/checkpoint policy and native PTY matrix.
  Review date: September 20, 2026.
- `TERM-012` remains Blocked, owned by Phase 0. Linux tests prove restoration
  from the ordinary baseline, but Crossterm cannot query every preexisting ANSI
  cursor/screen/paste mode. Removal requires an exact capture or documented
  ownership policy and tests for each observable initial state. Review date:
  September 20, 2026.
- Rust 1.88, Ubuntu 24.04, macOS 15, and Windows Server 2025 hosted jobs did not
  run locally. The fixed CI rows and Unix PTY jobs are definitions, not evidence.
- Windows ConPTY lifecycle, externally delivered signals, SSH/tmux, and named
  GUI terminal rows did not run and support is not claimed.

### Start the Rust foundation

**Commit subject:** `feat: start Rust foundation`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:15 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; rustc/cargo 1.97.1;
cargo-deny 0.20.2

Scope:

- Initialize the locked Rust package, application loop, terminal guard, base
  Ratatui shell, CLI, CI definition, and dependency policy.
- Exercise setup rollback, normal cleanup, cleanup after one restoration error,
  unwinding cleanup, state/focus transitions, deterministic rendering, and
  pre-terminal CLI behavior.

Selection:

- Changed paths: Cargo package and lockfile, application/CLI/terminal/UI source,
  tests, CI, dependency policy, README, plan, trackers, and test report.
- Classified areas: CLI/startup, terminal lifecycle, theme/UI foundation,
  dependency/feature flags, CI, license, and documentation.
- Selected case IDs: `QG-001` through `QG-005`, `QG-007` through `QG-013`,
  `CLI-001`, `CLI-002`, `CLI-005`, `TERM-001`, `TERM-003` through `TERM-005`,
  `TERM-007`, `PROP-010`, `SUP-001` through `SUP-004`, and `SUP-006` through
  `SUP-008`.
- Profiles run: the currently materialized local `pr-core` commands and
  dependency-policy check. The frozen machine-readable manifests do not exist
  yet and are not claimed complete.
- Fixtures: no book fixtures or downloaded corpus files were used.
- Environment: local non-PTY Linux process tests and Ratatui `TestBackend` at
  `40x10`; no native terminal compatibility row was claimed.

Checks:

| Command or procedure | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 10 unit/render tests and 3 CLI process tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed: advisories, bans, licenses, and sources |
| `cargo clean` | Passed: removed 1,444 files and 324.8 MiB |

Skipped or incomplete evidence:

- Rust 1.88 validation did not run because only Rust 1.97.1 is installed and
  `rustup` is unavailable in the local environment. A separate locked Rust 1.88
  CI job is defined but has not produced hosted evidence yet.
- Native PTY, Ctrl-C/signal, initial terminal-state capture, panic-diagnostic,
  SSH/tmux, macOS, and Windows restoration cases did not run. Current terminal
  case tests prove the guard logic with an injected control boundary only, so
  those case IDs are not promoted to Passing.
- The machine-readable case registry, executable profile and phase-gate
  manifests, fixture manifests, and full hermetic harness remain Phase 0 work.
- Hosted CI was defined but did not run locally. `SUP-004` has static workflow
  evidence only; release trigger and artifact controls remain later work.

### Define quality, testing, and UI implementation standards

**Commit subject:** `docs: define implementation standards`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:05 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64

Scope:

- Establish enforceable Rust implementation and review standards.
- Add a concrete catalog of stable test IDs, profiles, fixtures, environments,
  phase gates, selection rules, and completion criteria.
- Connect both standards to project planning and implementation tracking.
- Define exceptions, unresolved test decisions, and validation expectations.
- Add detailed ASCII UI mockups and implementation guidance.
- Reconcile the implementation roadmap and tracker while retaining six phases.

Selection:

- Changed paths: project standards, test catalog, UI specification, roadmap,
  tracker, README, decision log, and test report.
- Classified areas: documentation, test governance, UI hierarchy, phase gates,
  accessibility, and release evidence.
- Selected catalog IDs: `QG-010` for report completeness; no Rust implementation
  case was executable because the Cargo package and machine registry do not yet
  exist.
- Profiles: documentation validation only. Rust, render, PTY, security, fuzz,
  benchmark, native, and release profiles remain Planned.
- Fixtures and environments: no book fixture was opened; local Linux environment
  was used only for deterministic document validation.
- Blocked cases: the named `DEC-TEST-*` rows remain owned by their target phases
  with removal conditions stated in `testcases.md`; none was represented as
  passing.

Review passes:

| Pass | Focus | Incorporated results |
| --- | --- | --- |
| 1 | Locked feature coverage | Added default paged geometry, both-mode jumps, EPUB semantics, integrated images, ordered fallbacks, restore journeys, durable annotations, literal search, and exact theme/status/help requirements. |
| 2 | Security, privacy, and hostile input | Added exact boundary method, corrected atomic-save oracle, SVGZ/XML depth, hostile state paths, URL launch hardening, no-log inventory, filesystem/privacy canaries, decoder and supply-chain approval. |
| 3 | Terminal, platform, accessibility, and release | Added named environment rows, signal/initial-state restoration, native keyboard cases, real image-protocol cleanup, terminal Unicode/bidi limits, assistive technology matrix, benchmark method, and native release evidence. |
| 4 | Framework traceability and governance | Added case ownership/status lifecycle, machine registry, executable profile manifests, hermetic harness, fixture manifests, exact selection/report schema, immutable IDs, regression governance, and cumulative phase gates. |

Synchronization reviews:

| Review | Result |
| --- | --- |
| UI contract review | Added missing link focus, text selection, point/range note flow, search history, theme selection, status glossary, long-value inspection, mode-safe tiny-terminal recovery, confirmations, image anchor compensation, and explicit open UI decisions. |
| Six-phase roadmap review | Confirmed six phases remain correct; assigned registry/harness/UI shell to Phase 0 and strengthened the existing Phase 1-5 work and cumulative exit evidence. |

Checks:

| Check | Result |
| --- | --- |
| Standards cover correctness, security, privacy, testing, and maintainability | Passed |
| Required validation agrees with the project plan | Passed |
| Cargo cleanup and per-commit reporting requirements are explicit | Passed |
| Four independent completeness reviews completed and incorporated | Passed |
| Stable test case ID uniqueness | Passed: 332 definitions, no duplicates or dangling exact references |
| Markdown table and code-fence structure | Passed across eight files |
| Local Markdown links and table-of-contents entries | Passed across eight files |
| UI file contains ASCII only | Passed |
| Markdown files contain no em dash characters | Passed |
| Required Cargo command list agrees across standards and project plan | Passed |
| Six-phase count and numbering agree across plan, tracker, tests, and UI | Passed: `0,1,2,3,4,5` in each |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |

### Refine the planned Paper theme

**Commit subject:** `docs: refine Paper theme`

**Revision:** This commit

**Recorded:** August 19, 2026 at 10:16 PM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64

Scope:

- Define Paper's semantic palette and contrast floor.
- Specify responsive page behavior and terminal color fallbacks.
- Clarify image fidelity and measurable render coverage.

Checks:

| Check | Result |
| --- | --- |
| Paper requirements remain within the existing first-release theme scope | Passed |
| All seven planned foreground/background pairs exceed 4.5:1 calculated contrast | Passed: 5.14:1 minimum |
| Theme behavior explicitly preserves logical reading anchors | Passed |
| Source images remain unmodified and color-preserving by default | Passed |
| Local Markdown links resolve and changed headings retain their table-of-contents entries | Passed |
| Markdown files contain no em dash characters | Passed |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |

### Establish the local book corpus and test reporting

**Commit subject:** `docs: establish test reporting`

**Revision:** This commit

**Recorded:** August 19, 2026 at 7:32 PM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; curl 8.21.0;
Info-ZIP UnZip 6.00; file 5.48

Scope:

- Download three public-domain works in EPUB and plain-text forms.
- Keep downloaded reading material outside version control.
- Establish the required per-commit test reporting process.
- Add the six delivery phases to the operational implementation tracker.

Checks:

| Check | Result |
| --- | --- |
| Project Gutenberg source pages expose the selected downloads | Passed |
| `curl --fail --location` completed for all six files | Passed |
| `file` identifies three EPUB documents and three UTF-8 text files | Passed |
| `unzip -t` validates every member in all three EPUB archives | Passed |
| `sha256sum` recorded all six exact inputs | Passed |
| `git check-ignore -v` maps all six files to `.gitignore`'s `downloads/` rule | Passed |
| All local Markdown links added or retained by this change resolve | Passed |
| The implementation tracker lists all six project-plan delivery phases | Passed |
| Markdown files contain no em dash characters | Passed |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |
