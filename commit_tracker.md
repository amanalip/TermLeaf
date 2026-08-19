# Commit Tracker

**Last updated:** August 19, 2026 at 5:52 PM EDT

## Table of Contents

- [Purpose](#purpose)
- [Update Process](#update-process)
- [Pending Commit](#pending-commit)
- [Commit History](#commit-history)
- [Design Decision Log](#design-decision-log)

## Purpose

This file records the user-visible changes and design decisions associated
with each commit. It should be updated in the same change set as the work it
describes.

## Update Process

1. Add planned changes under **Pending Commit** while work is in progress.
2. Record decisions that affect architecture, behavior, or maintenance in the
   **Design Decision Log**.
3. Before committing, verify that the pending entry matches the staged diff.
4. Before committing, move the entry to **Commit History** with the intended
   commit subject and final timestamp, then verify both against Git history.

## Pending Commit

No changes are pending inclusion in a commit.

## Commit History

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
The initial plan therefore describes product capabilities and delivery gates
without prematurely selecting an implementation stack.

### DD-002: Track documentation beside source code

**Date:** August 19, 2026 at 5:48 PM EDT

**Status:** Accepted

Planning, implementation status, and commit context live at the repository
root so they are versioned, reviewable, and updated with related code changes.

### DD-003: Do not ignore dependency lockfiles

**Date:** August 19, 2026 at 5:48 PM EDT

**Status:** Accepted

Lockfiles are intentionally absent from `.gitignore`. Once a package manager
is selected, its lockfile should be committed to support reproducible builds.
