# Implementation Tracker

**Last updated:** August 19, 2026 at 10:16 PM EDT

## Table of Contents

- [How to Read This Tracker](#how-to-read-this-tracker)
- [Right Now](#right-now)
- [Delivery Phases](#delivery-phases)
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

| Phase | Status | Exit gate summary |
| --- | --- | --- |
| 0. Rust foundation | Not started | Native terminal restoration and locked CI pass. |
| 1. Plain-text reading loop | Not started | TXT reading, navigation, resize, and core test journeys pass. |
| 2. Structured books and images | Not started | Safe EPUB and Markdown reading plus image fallbacks pass. |
| 3. Dependable reading | Not started | Persistence, search, recents, annotations, help, and native terminal checks pass. |
| 4. Product refinement | Not started | Recovery, links, accessibility, performance, and reader guidance meet their gates. |
| 5. Release | Not started | Native packages, clean installs, checksums, notices, and platform smoke tests pass. |

## Groundwork

| Feature | Status | What remains |
| --- | --- | --- |
| Repository setup | Complete | The ignore rules and working documents are in place. |
| Project logo | Complete | The SVG mark is ready for documentation and future interfaces. |
| First-release features | Complete | The reader behavior, formats, images, annotations, themes, and platform intent are locked. |
| Remaining product details | In progress | Set the license expression, exact key map, OS versions, and tested terminals. |
| Stack selection | Complete | Rust, Ratatui, Crossterm, and the supporting crate strategy are documented. |
| Technical architecture | Complete | Module boundaries, data flow, security policy, and delivery gates are planned. |
| Rust package | Not started | Create the manifest, lockfile, module skeleton, and minimum Rust version. |
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
| Help screen | Not started | Show commands and active key bindings without leaving the reader. |
| Themes | Not started | Ship dark, light, high-contrast, monochrome, and a contrast-tested responsive Paper theme with true-color, 256-color, and terminal-default fallbacks. |
| Detailed status | Not started | Show title, chapter, location, page, percentage, clock, mode, and messages. |
| External links | Not started | Show destinations and confirm before opening the system browser. |
| Error messages | Not started | Say what failed, why it matters, and what the reader can try. |

## Confidence and Releases

| Feature | Status | What remains |
| --- | --- | --- |
| Automated tests | Not started | Cover parsing, navigation, state, and terminal behavior. |
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
