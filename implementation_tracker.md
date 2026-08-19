# Implementation Tracker

**Last updated:** August 19, 2026 at 6:50 PM EDT

## Table of Contents

- [How to Read This Tracker](#how-to-read-this-tracker)
- [Right Now](#right-now)
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

## Right Now

TermLeaf now has a researched Rust architecture and a staged delivery plan, but
no reader implementation yet. The next step is to initialize the Rust package,
prove terminal restoration on native platforms, and build the plain-text
reading loop before taking on EPUB parsing.

## Groundwork

| Feature | Status | What remains |
| --- | --- | --- |
| Repository setup | Complete | The ignore rules and working documents are in place. |
| Project logo | Complete | The SVG mark is ready for documentation and future interfaces. |
| Product boundaries | In progress | Confirm the support matrix, license expression, and default navigation. |
| Stack selection | Complete | Rust, Ratatui, Crossterm, and the supporting crate strategy are documented. |
| Technical architecture | Complete | Module boundaries, data flow, security policy, and delivery gates are planned. |
| Rust package | Not started | Create the manifest, lockfile, module skeleton, and minimum Rust version. |
| Command-line interface | Not started | Start with a book path and focused reader options through Clap. |
| Configuration | Not started | Implement defaults, TOML settings, and explicit CLI overrides. |

## The Reading Loop

| Feature | Status | What remains |
| --- | --- | --- |
| Plain-text rendering | Not started | Put readable text on screen without surprises. |
| Responsive layout | Not started | Wrap cleanly and recover when the terminal is resized. |
| Navigation | Not started | Move by line, page, chapter, start, and end. |
| Saved position | Not started | Reopen each book at the last stable location. |
| Search | Not started | Search in both directions without losing your place. |
| Plain-text format | Not started | Build the first complete reader path with bounded decoding. |
| EPUB format | Not started | Add bounded ZIP preflight, rbook semantics, and XHTML conversion. |

## The Bookshelf

| Feature | Status | What remains |
| --- | --- | --- |
| Open a local book | Not started | Explain bad paths, permissions, and unsupported files clearly. |
| Recent books | Not started | Make yesterday's book easy to reopen. |
| Library index | Not started | Add a local catalog only if it improves the reading flow. |
| Book details | Not started | Show title, author, and structure when the file provides them. |

## Terminal Experience

| Feature | Status | What remains |
| --- | --- | --- |
| Keyboard controls | Not started | Keep common actions quick and the full set discoverable. |
| Help screen | Not started | Show commands and active key bindings without leaving the reader. |
| Themes | Not started | Stay legible across terminal palettes and accessibility needs. |
| Error messages | Not started | Say what failed, why it matters, and what the reader can try. |

## Confidence and Releases

| Feature | Status | What remains |
| --- | --- | --- |
| Automated tests | Not started | Cover parsing, navigation, state, and terminal behavior. |
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
