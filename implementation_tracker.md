# Implementation Tracker

**Last updated:** August 30, 2026

## Table of Contents

- [How to Read This Tracker](#how-to-read-this-tracker)
- [Right Now](#right-now)
- [Delivery Phases](#delivery-phases)
- [UI Delivery](#ui-delivery)
- [Groundwork](#groundwork)
- [The Reading Loop](#the-reading-loop)
- [The Bookshelf](#the-bookshelf)
- [Terminal Experience](#terminal-experience)
- [Confidence and Releases](#confidence-and-releases)
- [Risks Worth Watching](#risks-worth-watching)

## How to Read This Tracker

| Status | What it means |
| --- | --- |
| Not started | Nobody has picked this up yet. |
| In progress | Work is under way. |
| Blocked | A specific choice or dependency has to land first. |
| Complete | The feature works, has been checked, and is documented. |
| Not planned | The feature is explicitly outside the current release. |

## Right Now

Most Phase 2 development is complete locally. Structured ingestion now covers
bounded ZIP/rbook EPUB semantics, semantic XHTML and Markdown, source-aware
links, exact internal chapter/fragment/TOC navigation, code/tables, immutable
archive reads, and structural gates for chapters plus every control-document
class. Raster and static SVG/SVGZ resources share byte, geometry, pixel,
allocation, active-content, resolver, and work limits. Terminal presentation
has positive-evidence backend selection, typed override conflicts,
Kitty/Sixel/iTerm2 serializers, true-color/256-color half-block frames, and
informative caption fallbacks. Native payloads use stable local IDs, bounded
chunks, post-Ratatui placement, replacement, navigation/resize invalidation, and
shutdown cleanup.
Two generation-aware workers enforce fixed queue and in-flight-byte budgets,
cancel stale work, contain failures, and shut down without blocking terminal
restoration. Wide TOC navigation preserves passage context in a side panel.
Exact Sixel pixel geometry, cancellation and allocation hardening, active
capability probing, and native-terminal acceptance remain before the transports
are complete. The local implementation suite and
security target pass; the Phase 2 gate remains evidence-in-progress until the
deterministic policy and manifests are reconciled, the remaining robustness and
hosted evidence passes, and native `IMG-018` results are recorded. DD-026
continues to own frozen members whose actual features land in later phases.

## Delivery Phases

**Implementation progress:** 2 of 6 phase gates complete. Phase 2 probing,
native acceptance, and external gate evidence are in progress.

The detailed work and exit gates remain in the
[project plan](project_plan.md#delivery-roadmap). This table is the operational
phase tracker and must be updated whenever work starts, completes, or becomes
blocked.

Each phase must pass its frozen exact gate, every earlier gate, and permanent
regressions. A required failure or Blocked P0 case prevents completion. Planning
documents marked Complete do not imply their Rust harness or feature is built.

| Phase | Status | Exit gate summary |
| --- | --- | --- |
| 0. Rust foundation | Complete | Implementation and local exact gate pass; hosted environment evidence remains recorded separately. |
| 1. Plain-text reading loop | Complete | Frozen gate passed locally and on ENV-LINUX-PTY, ENV-MAC-PTY, and ENV-WIN-PTY rows (run `32535725291`); cross-phase members owned forward by DD-026 with procedures recorded in `manual_procedures.md`. |
| 2. Structured books and images | In progress | Structured semantics/navigation, archive/control-document security, deterministic robustness, raster/SVG decoding, bounded workers, native and cell transports, and responsive TOC pass. Capability probing, hosted PTY rows, native protocol procedures, and gate evidence remain. |
| 3. Dependable reading | Not started | State, recents, search, selection, annotations, complete help, focus/text safety, and required native/accessibility evidence pass. |
| 4. Product refinement | Not started | Recovery, links, Paper matrix, privacy, usability, accessibility, performance, and guidance meet their gates. |
| 5. Release | Not started | Cumulative native, packaging/install, upgrade disposition, supply-chain, capture, and known-limitation evidence passes. |

## UI Delivery

`ui_mockups.md` is the hierarchy and interaction reference. It clarifies work
already inside the six phases; it does not add another phase.

| Phase | UI responsibility |
| --- | --- |
| 0 | View/focus state, action dispatch, terminal guard, base shell, test backend, case registry, and profile manifests |
| 1 | Reader modes, responsive classes, status foundation, core keys, all themes, Paper, help skeleton, errors, and too-small state |
| 2 | TOC, link focus, semantic code/tables, image placement/fallbacks, loading/cancellation, and resource errors |
| 3 | Recents, open path, search/history, text selection, annotation dialogs/editor/list, complete help, and persistence feedback |
| 4 | Metadata refinement, annotation recovery, links, long-value inspection, usability, accessibility, performance, and guidance |
| 5 | Native UI matrix, install journeys, terminal captures, limitations, and release documentation |

## Groundwork

| Feature | Status | What remains |
| --- | --- | --- |
| Repository setup | Complete | The ignore rules and working documents are in place. |
| Project logo | Complete | The SVG mark is ready for documentation and future interfaces. |
| First-release features | Complete | The reader behavior, formats, images, annotations, themes, and platform intent are locked. |
| Remaining product details | In progress | Phase 1 key map is set (hybrid conventional/Vim, `gg` prefix policy); OS support versions and tested release terminals remain. |
| Stack selection | Complete | Rust, Ratatui, Crossterm, and the supporting crate strategy are documented. |
| Technical architecture | Complete | Module boundaries, data flow, security policy, and delivery gates are planned. |
| Rust quality standards | Complete | Apply `code_quality.md` to implementation, review, testing, dependencies, and documented exceptions. |
| UI mockup specification | Complete | Implement responsive screens, focus states, overlays, accessibility, and phase ownership from `ui_mockups.md`. |
| Rust package | Complete | The manifest, committed lockfile, GPL expression, lint policy, and Rust 1.88 minimum are in place. |
| Application view/focus model | Complete | Every first-release view identity derives one exclusive focus owner; later phases add view-specific data and behavior. |
| Shared action registry | Complete | Foundation quit, interrupt, help, and back behavior share one registry; Phase 1 extends it with reader actions. |
| Command-line interface | Complete | Clap accepts an optional local book path and a `--theme` override, and handles help, version, missing, non-file, and unreadable paths before terminal setup. |
| Configuration | In progress | Defaults, TOML startup theme with default < config.toml < CLI precedence, tolerant fallbacks, and hermetic `XDG_CONFIG_HOME` relocation ship; full schema, typed errors, and state arrive with Phase 3 CFG cases. |

## The Reading Loop

| Feature | Status | What remains |
| --- | --- | --- |
| Plain-text rendering | Complete | Document model, bounded TXT decoding, wrapping layout with source mapping, viewport rendering, PTY render journeys, and the reviewed `tests/render.rs` assertion suite pass locally and on all hosted rows. |
| Responsive layout | Complete | Width-keyed layout cache with reuse/invalidation tests, cell-level Paper collapse order, resize transients over PTY, and deterministic property suites pass on the gate rows; real-terminal visual review joins the release matrix. |
| Navigation | In progress | Line/page/start/end/section steps move one validated anchor with clamped boundaries; the TOC overlay jumps sections on Enter with exact return stacking; search-result jumps arrive with later phases. |
| Saved position | Not started | Reopen each book at the last stable location. |
| Search | Not started | Search in both directions with smart-case matching and visible results. |
| Plain-text format | Complete | BOM detection, strict UTF-8, marked UTF-16, UTF-32 rejection, newline normalization, paragraph preservation, exact byte limits, hostile fixtures, fixed-seed raw-byte properties, and bounded mutations produce a valid model or typed bounded error. |
| EPUB format | Complete | Bounded archive and control-document gates, package/spine/metadata semantics, canonical reading order, tolerant semantic XHTML, exact internal links/TOC positions, encryption/fixed-layout errors, lazy resources, no-extraction evidence, and immutable inspected bytes pass locally. |
| Markdown format | Complete | Source-aware parsing maps full block/inline semantics, code languages and copy ranges, inert link targets, original source ranges, and images into the shared bounded model. |
| Inline images | In progress | Raster plus static SVG/SVGZ decode, hostile-resource rejection, generation-aware worker bounds, positive-evidence backend selection, half-block cells, Paper-safe pixels, and caption errors pass locally. Native protocol procedure and deterministic robustness evidence remain. |

## The Bookshelf

| Feature | Status | What remains |
| --- | --- | --- |
| Open a local book | In progress | Extension-first detection (DD-024/`DEC-TEST-001`) accepts case-insensitive `.txt` and `.epub` and rejects other or missing extensions pre-terminal; TXT content decodes strictly and EPUB content passes the bounded archive layer before parsing; missing, non-file, unreadable, oversized, undecodable, and misleading-content paths fail before terminal setup. |
| Recent books | Not started | Reopen, remove, and clear recent entries without scanning directories. |
| Library index | Not planned | Keep automatic indexing outside the first release. |
| Book details | Not started | Show title, author, and structure when the file provides them. |
| Bookmarks | Not started | Create, name, list, jump to, rename, and delete stable bookmarks. |
| Highlights | Not started | Store accessible colored ranges outside the source book. |
| Notes | Not started | Attach editable local text to logical passages. |

## Terminal Experience

| Feature | Status | What remains |
| --- | --- | --- |
| Keyboard controls | Complete | Full KEY-001 matrix, flow-control paging, paste inertness, Escape/Alt scope, and resize journeys pass over native PTYs on all hosted rows; manual GUI halves are documented in `manual_procedures.md` and owned by the release matrix (DD-026). |
| Open-path screen | Not started | Accept typed/pasted local paths with focused validation and no directory scanning. |
| Table of contents | Complete | Standard/narrow screens use the full-screen list; wide screens retain passage context beside a TOC panel. Both jump exact validated navigation points with cursor memory and confirmation messages. |
| Responsive UI states | In progress | Wide/standard/compact/narrow classes drive Paper chrome and status collapse; below-minimum suspends safely and recovers; full matrix review remains. |
| Loading and cancellation UI | In progress | Bounded generations, cancellation, stale-result rejection, fallback states, and anchor-independent work coordination pass at the model boundary; final protocol-specific native presentation evidence remains. |
| Help screen | In progress | Skeleton lists every registered binding and returns to the invoking view; contextual/mode-scoped help arrives later. |
| Themes | Complete | All five themes ship with semantic roles, `NO_COLOR` fallback, session switching, tested Paper contrast, TOML-backed startup choice with CLI override, nearest-256 output, a three-mode by five-viewport capability matrix, true-color role checks, and anchor-preserving switches mid-passage; real-terminal visual review joins the Deferred release rows. |
| Detailed status | In progress | Priority-ordered collapse, floored percent, logical location, dynamic page, UTC clock, tick-lifetime messages on every screen including the home status, and theme confirmations ship; failed-save states arrive with persistence. |
| External links | Not started | Show destinations and confirm before opening the system browser. |
| Error messages | Complete | Typed document errors name path, reason, and recovery before terminal setup, and diagnostics escape control bytes so hostile paths cannot inject terminal sequences; the in-app recoverable-error view is later-phase scope. |

## Confidence and Releases

| Feature | Status | What remains |
| --- | --- | --- |
| Automated tests | In progress | Foundation, reading-loop, property, render, PTY, archive-security, and image-decode suites run — 199 library, 9 CLI, 17 document-I/O, 16 render, 8 property, 9 security, and 14 native PTY cases locally; hosted rows for the gate revision remain to be recorded. |
| Test framework specification | Complete | Use stable IDs, exact profiles, fixtures, environments, phase gates, and blocked-decision rules from `testcases.md`. |
| Machine-readable case registry | Complete | All 336 IDs, owners, profiles, resources, locations, status overrides, and evidence links validate bidirectionally. |
| Executable profile manifests | Complete | Exact core, render, PTY, security, scheduled, weekly, and release commands and memberships are versioned; `FUZZ-*` targets are optional weekly discovery metadata rather than required security work. |
| Frozen cumulative phase gates | Complete | Phase 0 through Phase 5 membership is frozen as exact cumulative deterministic IDs and required native environment rows; fuzz IDs and default durations are prohibited. |
| Structured fixture corpus | Complete | Twenty-nine authored TXT, Markdown, EPUB, per-decoder raster, SVG, and SVGZ files reproduce byte-for-byte and carry hashes, sizes, provenance, SPDX licenses, generator parameters, expected properties, and served case IDs. |
| Hermetic test harness | Complete | Foundation CLI and PTY cases isolate user paths/environment, dimensions, terminal model, child cleanup, faults, and deadlines; later boundaries extend the same contract. |
| Test reporting | Complete | Record exact checks, outcomes, skipped coverage, fixtures, and cleanup for every commit in `testreport.md`. |
| Continuous integration | In progress | Fixed Linux, macOS, and Windows core jobs, Unix PTY jobs, MSRV, and dependency policy are defined but need hosted evidence for this change. |
| Dependency policy | In progress | `deny.toml` covers advisories, licenses, sources, and bans and passes locally with the Phase 2 dependencies (adding `ISC` for scraper alongside `BSD-3-Clause` for bundled WHATWG data and `MPL-2.0` for `option-ext`, plus the `image` 0.25 format graph with a documented `RUSTSEC-2024-0436` paste exception arriving through the locked OpenEXR decoder); hosted and cross-platform evidence remains. |
| Packaging | Not started | Choose channels that fit the supported platforms. |
| Release routine | Not started | Make versioning and artifact creation repeatable. |
| Reader documentation | In progress | Keep instructions useful as the application takes shape. |

## Risks Worth Watching

| Risk | What could go wrong | How we reduce it |
| --- | --- | --- |
| Hostile EPUB archives | A small input consumes unreasonable memory or processing time. | Enforce archive, entry, decompression, and parser limits before reading content. |
| Terminals behave differently | Layout or controls work locally but fail for readers elsewhere. | Name supported terminals and exercise them in integration tests. |
| Large books strain memory | Opening or moving through a book becomes noticeably slow. | Set performance budgets and test large files from the first reader slice. |
| Saved state is damaged | Readers lose their place or cannot reopen a book. | Write state atomically and version its on-disk shape. |
| Unicode layout drifts | Width, wrapping, search, and highlights disagree. | Keep logical positions separate from visual rows and test grapheme-safe mappings. |
| Image capability detection fails | Escape sequences leak or an image covers stale terminal content. | Require positive protocol evidence, allow overrides, and always retain cell and caption fallbacks. |
| Image decoding exhausts resources | A crafted image consumes excessive memory or CPU. | Enforce byte, dimension, pixel, resolver, allocation, and queue limits before rendering. |
| Loose image paths change during open | A Markdown directory modified concurrently swaps a checked path before the worker opens it. | Open relative to a persistent capability directory; static and concurrent symlink-escape tests prove outside files remain unreachable. |
