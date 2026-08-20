# Implementation Tracker

**Last updated:** August 20, 2026 at 12:05 AM EDT

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

TermLeaf now has a researched Rust architecture and a locked first-release
feature contract, but no reader implementation yet. The next step is to
initialize the Rust package, prove terminal restoration on native platforms,
and build the plain-text reading loop before structured formats and images.

## Delivery Phases

**Overall progress:** 0 of 6 phases complete. Phase 0 is next.

The detailed work and exit gates remain in the
[project plan](project_plan.md#delivery-roadmap). This table is the operational
phase tracker and must be updated whenever work starts, completes, or becomes
blocked.

Each phase must pass its frozen exact gate, every earlier gate, and permanent
regressions. A required failure or Blocked P0 case prevents completion. Planning
documents marked Complete do not imply their Rust harness or feature is built.

| Phase | Status | Exit gate summary |
| --- | --- | --- |
| 0. Rust foundation | Not started | UI shell, terminal guard, action/view state, locked CI, registry, profiles, harness, and exact foundation gate pass. |
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
| Remaining product details | In progress | Set the license expression, exact key map, OS versions, and tested terminals. |
| Stack selection | Complete | Rust, Ratatui, Crossterm, and the supporting crate strategy are documented. |
| Technical architecture | Complete | Module boundaries, data flow, security policy, and delivery gates are planned. |
| Rust quality standards | Complete | Apply `code_quality.md` to implementation, review, testing, dependencies, and documented exceptions. |
| UI mockup specification | Complete | Implement responsive screens, focus states, overlays, accessibility, and phase ownership from `ui_mockups.md`. |
| Rust package | Not started | Create the manifest, lockfile, module skeleton, and minimum Rust version. |
| Application view/focus model | Not started | Represent recent, open-path, reader, selection, editor, confirmation, help, error, and too-small states explicitly. |
| Shared action registry | Not started | Drive input handling and generated help from one conflict-free action/key source. |
| Command-line interface | Not started | Start with a book path and focused reader options through Clap. |
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
| Automated tests | Not started | Cover parsing, navigation, state, and terminal behavior. |
| Test framework specification | Complete | Use stable IDs, exact profiles, fixtures, environments, phase gates, and blocked-decision rules from `testcases.md`. |
| Machine-readable case registry | Not started | Materialize IDs, status, ownership, implementation links, environments, fixtures, and evidence in Phase 0. |
| Executable profile manifests | Not started | Implement `pr-core`, render, PTY, security, scheduled, phase, and release profiles in Phase 0. |
| Frozen cumulative phase gates | Not started | Expand broad planning families into exact IDs and environments beginning with `phase-gate-0`. |
| Hermetic test harness | Not started | Isolate paths, environment, time, network, workers, terminal state, fixtures, and fault injection. |
| Test reporting | Complete | Record exact checks, outcomes, skipped coverage, fixtures, and cleanup for every commit in `testreport.md`. |
| Continuous integration | Not started | Check style, tests, and builds on every proposed change. |
| Dependency policy | Not started | Configure cargo-deny for advisories, licenses, sources, and bans. |
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
