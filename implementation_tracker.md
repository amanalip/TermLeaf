# Implementation Tracker

**Last updated:** August 20, 2026 at 1:02 AM EDT

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

Phase 0 implementation is complete. The 336-case registry has no incomplete
foundation case, the view/focus and action loop is explicit, unreadable paths
fail before terminal setup, supported interrupts exit cleanly, and the native
PTY target covers the documented shell baseline. Phase 1 is the next development
phase. Hosted MSRV and platform jobs remain formal environment evidence.

## Delivery Phases

**Implementation progress:** 1 of 6 phases complete. Phase 1 is next.

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
| 1. Plain-text reading loop | Not started | Responsive TXT reader, modes, keys, status, all themes, help skeleton, errors, and anchor-preserving render/PTY evidence pass. |
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
| Remaining product details | In progress | Set the Phase 1 key map, OS support versions, and tested release terminals. |
| Stack selection | Complete | Rust, Ratatui, Crossterm, and the supporting crate strategy are documented. |
| Technical architecture | Complete | Module boundaries, data flow, security policy, and delivery gates are planned. |
| Rust quality standards | Complete | Apply `code_quality.md` to implementation, review, testing, dependencies, and documented exceptions. |
| UI mockup specification | Complete | Implement responsive screens, focus states, overlays, accessibility, and phase ownership from `ui_mockups.md`. |
| Rust package | Complete | The manifest, committed lockfile, GPL expression, lint policy, and Rust 1.88 minimum are in place. |
| Application view/focus model | Complete | Every first-release view identity derives one exclusive focus owner; later phases add view-specific data and behavior. |
| Shared action registry | Complete | Foundation quit, interrupt, help, and back behavior share one registry; Phase 1 extends it with reader actions. |
| Command-line interface | Complete | Clap accepts an optional local book path and handles help, version, missing, non-file, and unreadable paths before terminal setup. |
| Configuration | Not started | Implement defaults, TOML settings, and explicit CLI overrides. |

## The Reading Loop

| Feature | Status | What remains |
| --- | --- | --- |
| Plain-text rendering | Not started | Put readable text on screen without surprises. |
| Responsive layout | Not started | Wrap cleanly in paged and continuous modes while preserving the logical anchor. |
| Navigation | Not started | Support conventional and Vim-style line, page, chapter, start, and end controls. |
| Saved position | Not started | Reopen each book at the last stable location. |
| Search | Not started | Search in both directions with smart-case matching and visible results. |
| Plain-text format | Not started | Build the first complete reader path with bounded decoding. |
| EPUB format | Not started | Add bounded ZIP preflight, rbook semantics, and XHTML conversion. |
| Markdown format | Not started | Parse source-aware Markdown directly into the shared document model. |
| Inline images | Not started | Decode bounded raster and SVG content through protocol, cell, and caption fallbacks. |

## The Bookshelf

| Feature | Status | What remains |
| --- | --- | --- |
| Open a local book | Not started | Explain bad paths, permissions, and unsupported files clearly. |
| Recent books | Not started | Reopen, remove, and clear recent entries without scanning directories. |
| Library index | Not planned | Keep automatic indexing outside the first release. |
| Book details | Not started | Show title, author, and structure when the file provides them. |
| Bookmarks | Not started | Create, name, list, jump to, rename, and delete stable bookmarks. |
| Highlights | Not started | Store accessible colored ranges outside the source book. |
| Notes | Not started | Attach editable local text to logical passages. |

## Terminal Experience

| Feature | Status | What remains |
| --- | --- | --- |
| Keyboard controls | Not started | Keep common actions quick and the full set discoverable. |
| Open-path screen | Not started | Accept typed/pasted local paths with focused validation and no directory scanning. |
| Table of contents | Not started | Provide contextual side-panel or full-screen chapter navigation. |
| Responsive UI states | Not started | Support wide, standard, compact, narrow, and non-destructive below-minimum layouts. |
| Loading and cancellation UI | Not started | Show bounded static progress and preserve anchors when stale work is canceled. |
| Help screen | Not started | Show commands and active key bindings without leaving the reader. |
| Themes | Not started | Ship dark, light, high-contrast, monochrome, and a contrast-tested responsive Paper theme with true-color, 256-color, and terminal-default fallbacks. |
| Detailed status | Not started | Show title, chapter, location, page, percentage, clock, mode, and messages. |
| External links | Not started | Show destinations and confirm before opening the system browser. |
| Error messages | Not started | Say what failed, why it matters, and what the reader can try. |

## Confidence and Releases

| Feature | Status | What remains |
| --- | --- | --- |
| Automated tests | In progress | Foundation unit, render, isolated CLI, and Linux PTY tests run; native platform and later feature suites remain. |
| Test framework specification | Complete | Use stable IDs, exact profiles, fixtures, environments, phase gates, and blocked-decision rules from `testcases.md`. |
| Machine-readable case registry | Complete | All 336 IDs, owners, profiles, resources, locations, status overrides, and evidence links validate bidirectionally. |
| Executable profile manifests | Complete | Exact core, render, PTY, security, scheduled, weekly, and release commands and memberships are versioned; later targets activate with their phases. |
| Frozen cumulative phase gates | Complete | Phase 0 through Phase 5 membership is frozen as exact cumulative IDs and required native environment rows. |
| Hermetic test harness | Complete | Foundation CLI and PTY cases isolate user paths/environment, dimensions, terminal model, child cleanup, faults, and deadlines; later boundaries extend the same contract. |
| Test reporting | Complete | Record exact checks, outcomes, skipped coverage, fixtures, and cleanup for every commit in `testreport.md`. |
| Continuous integration | In progress | Fixed Linux, macOS, and Windows core jobs, Unix PTY jobs, MSRV, and dependency policy are defined but need hosted evidence for this change. |
| Dependency policy | In progress | `deny.toml` covers advisories, licenses, sources, and bans and passes locally; hosted and cross-platform evidence remains. |
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
