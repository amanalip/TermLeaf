# TermLeaf

<p align="center">
  <img src="assets/termleaf-logo.svg" alt="TermLeaf logo" width="180">
</p>

Turn pages without leaving the terminal.

**Last updated:** August 19, 2026 at 7:18 PM EDT

## Table of Contents

- [What Is TermLeaf?](#what-is-termleaf)
- [First Release](#first-release)
- [Where Things Stand](#where-things-stand)
- [Technical Direction](#technical-direction)
- [Project Notes](#project-notes)

## What Is TermLeaf?

TermLeaf is a reader for people who are happiest at the command line. Open a
book, settle into a clean reading view, and pick up where you stopped last
time. No browser tab, mouse hunt, or account should get in the way.

## First Release

TermLeaf will open local TXT, Markdown, and reflowable EPUB books. The reader
will include:

- Paged and continuous reading modes.
- Semantic text and best-effort terminal images.
- Conventional and Vim-style navigation keys.
- Smart-case search and detailed reading progress.
- Saved positions, bookmarks, highlights, and notes.
- A recent-books screen without automatic directory scanning.
- Dark, light, high-contrast, monochrome, and Paper themes.
- Confirmed external links and in-application help.
- Native Linux, macOS, and Windows releases after platform tests pass.

Images will use a supported terminal graphics protocol when one is positively
detected, fall back to a cell-based preview, and finally show a useful caption.
Books and annotations remain local, and TermLeaf never rewrites the source book.

## Where Things Stand

The product contract and technical architecture are settled, but implementation
has not started. The next milestone is the Rust foundation and a complete
plain-text reading loop with reliable terminal restoration.

## Technical Direction

TermLeaf will use stable Rust with Ratatui and Crossterm. Plain text comes first,
followed by Markdown and bounded EPUB parsing. The reader owns logical document
positions and Unicode-aware layout so resizing does not lose the current
passage. The [project plan](project_plan.md) records the complete architecture,
security limits, delivery gates, and primary references.

## Project Notes

- [Project plan](project_plan.md): where TermLeaf is going and how we will get
  there.
- [Implementation tracker](implementation_tracker.md): what is built, what is
  next, and what is holding us up.
- [Commit tracker](commit_tracker.md): what changed and the reasoning behind
  decisions that will matter later.
