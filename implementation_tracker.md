# Implementation Tracker

**Last updated:** August 21, 2026 at 11:59 PM EDT

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

The phase-gate-1 evidence run is complete locally. The frozen manifest's
closeable members all pass: the full reader key matrix, bracketed-paste
inertness, resize transients, focus/mouse/release event inertness, the
deterministic property suite, extension-first format detection under the
resolved `DEC-TEST-001` (DD-025), locale variants, right-to-left samples,
ambiguous-width policy, the Paper capability matrix and collapse order,
status collapse stepping, redraw stability, help from every Phase 1 surface,
and typed-error diagnostics with control-byte escaping. One navigation
defect found by the properties was fixed: previous page is now the exact
inverse of next page whenever the forward hop fits unclamped. Cross-phase
members are owned forward by DD-026 with procedures written in
`manual_procedures.md`. Remaining before Complete: pushed hosted CI rows for
this revision and the release-matrix manual executions that require real GUI
terminals.

## Delivery Phases

**Implementation progress:** 1 of 6 phases complete. Phase 1 is in progress.

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
| 1. Plain-text reading loop | In progress | Implementation, local core suite, and PTY journeys pass; hosted rows, manual procedures, render review, and the full gate run remain. |
| 2. Structured books and images | Not started | Safe Markdown/EPUB semantics, TOC, code/tables, images, loading/cancellation, security boundaries, and fuzz evidence pass. |
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
| Plain-text rendering | In progress | Document model, bounded TXT decoding, wrapping layout with source mapping, viewport rendering, PTY render journeys, and the reviewed `tests/render.rs` assertion suite work; hosted rows for the gate revision remain. |
| Responsive layout | In progress | Width-keyed layout cache with reuse/invalidation tests, cell-level Paper collapse order, resize transients over PTY, and deterministic property suites all pass; release-matrix visual checks remain. |
| Navigation | In progress | Line/page/start/end/section steps move one validated anchor with clamped boundaries; TOC/search jumps arrive with later phases. |
| Saved position | Not started | Reopen each book at the last stable location. |
| Search | Not started | Search in both directions with smart-case matching and visible results. |
| Plain-text format | In progress | BOM detection, strict UTF-8, marked UTF-16, newline normalization, paragraph preservation, and file-level size-limit integration evidence are done; fuzz coverage arrives with the security profile. |
| EPUB format | Not started | Add bounded ZIP preflight, rbook semantics, and XHTML conversion. |
| Markdown format | Not started | Parse source-aware Markdown directly into the shared document model. |
| Inline images | Not started | Decode bounded raster and SVG content through protocol, cell, and caption fallbacks. |

## The Bookshelf

| Feature | Status | What remains |
| --- | --- | --- |
| Open a local book | In progress | Extension-first detection (DD-024/`DEC-TEST-001`) accepts case-insensitive `.txt` and rejects other or missing extensions pre-terminal; missing, non-file, unreadable, oversized, undecodable, and misleading-content paths fail before terminal setup. |
| Recent books | Not started | Reopen, remove, and clear recent entries without scanning directories. |
| Library index | Not planned | Keep automatic indexing outside the first release. |
| Book details | Not started | Show title, author, and structure when the file provides them. |
| Bookmarks | Not started | Create, name, list, jump to, rename, and delete stable bookmarks. |
| Highlights | Not started | Store accessible colored ranges outside the source book. |
| Notes | Not started | Attach editable local text to logical passages. |

## Terminal Experience

| Feature | Status | What remains |
| --- | --- | --- |
| Keyboard controls | In progress | Full KEY-001 matrix, flow-control paging, paste inertness, Escape/Alt scope, and resize journeys pass over native PTYs; manual GUI halves are documented in `manual_procedures.md` and owned by the release matrix (DD-026). |
| Open-path screen | Not started | Accept typed/pasted local paths with focused validation and no directory scanning. |
| Table of contents | Not started | Provide contextual side-panel or full-screen chapter navigation. |
| Responsive UI states | In progress | Wide/standard/compact/narrow classes drive Paper chrome and status collapse; below-minimum suspends safely and recovers; full matrix review remains. |
| Loading and cancellation UI | Not started | Show bounded static progress and preserve anchors when stale work is canceled. |
| Help screen | In progress | Skeleton lists every registered binding and returns to the invoking view; contextual/mode-scoped help arrives later. |
| Themes | In progress | All five themes ship with semantic roles, `NO_COLOR` fallback, session switching, tested Paper contrast, TOML-backed startup choice with CLI override, nearest-256 output, a three-mode by five-viewport capability matrix, true-color role checks, and anchor-preserving switches mid-passage; real-terminal visual review belongs to the Deferred release rows. |
| Detailed status | In progress | Priority-ordered collapse, floored percent, logical location, dynamic page, UTC clock, tick-lifetime messages on every screen including the home status, and theme confirmations ship; failed-save states arrive with persistence. |
| External links | Not started | Show destinations and confirm before opening the system browser. |
| Error messages | In progress | Typed document errors name path, reason, and recovery before terminal setup, and diagnostics escape control bytes so hostile paths cannot inject terminal sequences; the in-app recoverable-error view and note-content half arrive later. |

## Confidence and Releases

| Feature | Status | What remains |
| --- | --- | --- |
| Automated tests | In progress | Foundation, reading-loop, property, render, and PTY suites run — 97 library, 8 CLI, 4 document-I/O, 14 render, 6 property, and 14 native PTY cases locally; hosted rows for the gate revision remain to be recorded. |
| Test framework specification | Complete | Use stable IDs, exact profiles, fixtures, environments, phase gates, and blocked-decision rules from `testcases.md`. |
| Machine-readable case registry | Complete | All 336 IDs, owners, profiles, resources, locations, status overrides, and evidence links validate bidirectionally. |
| Executable profile manifests | Complete | Exact core, render, PTY, security, scheduled, weekly, and release commands and memberships are versioned; later targets activate with their phases. |
| Frozen cumulative phase gates | Complete | Phase 0 through Phase 5 membership is frozen as exact cumulative IDs and required native environment rows. |
| Hermetic test harness | Complete | Foundation CLI and PTY cases isolate user paths/environment, dimensions, terminal model, child cleanup, faults, and deadlines; later boundaries extend the same contract. |
| Test reporting | Complete | Record exact checks, outcomes, skipped coverage, fixtures, and cleanup for every commit in `testreport.md`. |
| Continuous integration | In progress | Fixed Linux, macOS, and Windows core jobs, Unix PTY jobs, MSRV, and dependency policy are defined but need hosted evidence for this change. |
| Dependency policy | In progress | `deny.toml` covers advisories, licenses, sources, and bans and passes locally with the Phase 1 dependencies (including `BSD-3-Clause` for bundled WHATWG data and `MPL-2.0` for `option-ext`); hosted and cross-platform evidence remains. |
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
