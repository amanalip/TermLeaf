# Test Report

**Last updated:** August 19, 2026 at 10:16 PM EDT

## Table of Contents

- [Purpose](#purpose)
- [Update Rules](#update-rules)
- [Local Gutenberg Corpus](#local-gutenberg-corpus)
- [Pending Commit](#pending-commit)
- [Commit Reports](#commit-reports)

## Purpose

This file records the evidence behind each TermLeaf commit. It distinguishes
checks that passed from checks that were skipped, unavailable, or not yet
applicable. It does not replace CI output, fuzzing artifacts, benchmark data, or
platform release records.

## Update Rules

Every commit must have one report entry that records:

- The commit subject and revision, using `This commit` when the report is part
  of the commit it describes or `Pending` before its subject is chosen.
- The behavior and risks exercised by the checks.
- The operating system and relevant tool versions.
- Each command or manual procedure and its result.
- Fixtures used, including provenance when they are not generated locally.
- Skipped or unavailable checks and the reason.
- Whether `cargo clean` ran after the complete local Rust validation cycle.

Never mark a check as passing when it was not run. Prepare the report with the
change and identify that atomic change as `This commit`; requiring its own hash
inside its contents would make the hash self-referential. Keep detailed CI logs
in CI rather than copying large logs into this file.

## Local Gutenberg Corpus

The following books were downloaded explicitly for development from Project
Gutenberg on August 19, 2026. They are public domain in the USA according to
their source pages. The files remain under ignored `downloads/gutenberg/` and
are not repository assets.

| Work | Gutenberg ID | Local formats | Purpose |
| --- | ---: | --- | --- |
| Alice's Adventures in Wonderland | 11 | EPUB 3 with images, UTF-8 TXT | EPUB 3 navigation, XHTML, illustrations, and text comparison |
| Frankenstein | 84 | Legacy EPUB without book illustrations, UTF-8 TXT | Longer chapter sequence and EPUB/TXT comparison |
| Pride and Prejudice | 1342 | Legacy EPUB without book illustrations, UTF-8 TXT | Larger prose document, chapter navigation, and sustained layout |

Download sources:

- `https://www.gutenberg.org/ebooks/11`
- `https://www.gutenberg.org/ebooks/84`
- `https://www.gutenberg.org/ebooks/1342`

The current local files occupy 2,499,711 bytes in total. Their SHA-256 values
identify the exact inputs used in this report:

| Local file | Bytes | SHA-256 |
| --- | ---: | --- |
| `alice-11.epub` | 189,231 | `6b79f2d23b804172816e81c463dbcea689593bbde63ef200d52b6c0da7ef629c` |
| `alice-11.txt` | 174,311 | `01b38ea4c710a84bc18d0bd41271a5a1a92b94e97b2812f4dece97d4a694725e` |
| `frankenstein-84.epub` | 356,351 | `2719565ac885c335df88f220b03a9c45b95dc4225193a8dc649f6493550c4332` |
| `frankenstein-84.txt` | 448,885 | `7810cd483cffcf2cc8a1d8f0d5807931e69d4f48cd14149b8c76f88af82fead3` |
| `pride-and-prejudice-1342.epub` | 558,547 | `462be7852d84412c6695851395144a97e9762d45bd3c41b9f356dc7ac047b8a9` |
| `pride-and-prejudice-1342.txt` | 772,386 | `74f2665d6e6925fc2c17dec644bec9e87df478a0f1836822125e8acbb3777806` |

Real books exercise ordinary document structure. Purpose-built fixtures must
still cover malformed archives, path traversal, compression bombs, excessive
resource use, invalid encoding, hostile SVG content, and other security limits.

## Pending Commit

No additional changes are pending inclusion in a commit.

## Commit Reports

### Refine the planned Paper theme

**Commit subject:** `docs: refine Paper theme`

**Revision:** This commit

**Recorded:** August 19, 2026 at 10:16 PM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64

Scope:

- Define Paper's semantic palette and contrast floor.
- Specify responsive page behavior and terminal color fallbacks.
- Clarify image fidelity and measurable render coverage.

Checks:

| Check | Result |
| --- | --- |
| Paper requirements remain within the existing first-release theme scope | Passed |
| All seven planned foreground/background pairs exceed 4.5:1 calculated contrast | Passed: 5.14:1 minimum |
| Theme behavior explicitly preserves logical reading anchors | Passed |
| Source images remain unmodified and color-preserving by default | Passed |
| Local Markdown links resolve and changed headings retain their table-of-contents entries | Passed |
| Markdown files contain no em dash characters | Passed |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |

### Establish the local book corpus and test reporting

**Commit subject:** `docs: establish test reporting`

**Revision:** This commit

**Recorded:** August 19, 2026 at 7:32 PM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; curl 8.21.0;
Info-ZIP UnZip 6.00; file 5.48

Scope:

- Download three public-domain works in EPUB and plain-text forms.
- Keep downloaded reading material outside version control.
- Establish the required per-commit test reporting process.
- Add the six delivery phases to the operational implementation tracker.

Checks:

| Check | Result |
| --- | --- |
| Project Gutenberg source pages expose the selected downloads | Passed |
| `curl --fail --location` completed for all six files | Passed |
| `file` identifies three EPUB documents and three UTF-8 text files | Passed |
| `unzip -t` validates every member in all three EPUB archives | Passed |
| `sha256sum` recorded all six exact inputs | Passed |
| `git check-ignore -v` maps all six files to `.gitignore`'s `downloads/` rule | Passed |
| All local Markdown links added or retained by this change resolve | Passed |
| The implementation tracker lists all six project-plan delivery phases | Passed |
| Markdown files contain no em dash characters | Passed |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |
