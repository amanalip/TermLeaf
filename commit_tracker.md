# Commit Tracker

**Last updated:** August 19, 2026 at 7:32 PM EDT

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

No changes are pending inclusion in a commit.

## Commit History

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
fallbacks. It will not fake texture or reduce contrast for decoration.
