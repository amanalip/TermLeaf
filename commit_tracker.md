# Commit Tracker

**Last updated:** August 19, 2026 at 6:22 PM EDT

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
