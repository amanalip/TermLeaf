# Test Report

**Last updated:** August 20, 2026 at 1:02 AM EDT

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
- Changed paths and their classified areas from `testcases.md`.
- Exact selected case IDs, profile manifests, commands, features, environments,
  fixture hashes, generated seeds, and external evidence links.
- For each Blocked case: owner, reason, compensating evidence, removal condition,
  and review date.

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

### Complete Phase 0 implementation

**Commit subject:** `feat: complete Phase 0 implementation`

**Revision:** This commit

**Recorded:** August 20, 2026 at 1:02 AM EDT

Scope:

- Complete first-release view/focus identities and foundation action-state
  invariants without implementing later screen behavior early.
- Retain one read-only source handle and reject unreadable input before terminal
  initialization.
- Resolve `DEC-TEST-013` with a bounded interrupt family and document the
  supported shell launch baseline for write-only ANSI modes.
- Generalize the native PTY target for Unix and Windows/ConPTY, including raw
  Ctrl-C, external `SIGINT`, and captured native terminal attributes.
- Add exact `APP-001` through `APP-004` cases and remove broad later-phase cases
  from the Phase 0 gate.

Selection:

- Changed areas: application state, CLI/startup, terminal lifecycle, process
  interrupts, native PTY tests, dependencies, test governance, and documentation.
- Selected case IDs: all exact `phase-gate-0` IDs in
  `tests/phase_gates.toml`, including `APP-001` through `APP-004`, `CLI-006`,
  `TERM-011`, and `TERM-012`.
- Profiles run: registry freshness, `pr-core`, local native PTY, and dependency
  policy.
- Fixtures: synthetic temporary files and case-owned PTYs only; no book corpus
  fixture was opened.
- Environment: Linux 7.1.8-1-cachyos x86-64, Rust 1.97.1, Python 3.14.7,
  cargo-deny 0.20.2, and `TERM=xterm-256color` inside `80x24` PTYs.

Checks:

| Command or procedure | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Passed: 336 unique IDs and no Phase 0 case lacks implementation evidence |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 17 library, 4 CLI, and 7 native PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed after interrupt and Unix signal test dependency additions |
| `git diff --check` | Passed |
| `cargo clean` | Passed: removed 2,595 files and 485.7 MiB |

Unavailable environment evidence:

- Rust 1.88, Ubuntu 24.04, macOS 15, and Windows Server 2025/ConPTY jobs cannot
  run in this local environment. Their fixed CI definitions remain the formal
  evidence path and do not leave a Phase 0 implementation case incomplete.

### Continue the Rust foundation

**Commit subject:** `test: add Phase 0 manifests and PTY harness`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:44 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; rustc/cargo 1.97.1;
Python 3.14.7; cargo-deny 0.20.2; `TERM=xterm-256color` in PTY cases

Scope:

- Materialize and validate all 332 catalog IDs, exact profile assignments,
  fixture/environment references, test locations, and cumulative gate members.
- Isolate CLI and PTY processes with case-owned paths, minimal environments,
  fixed `80x24` PTYs, VT100 parsing, kernel terminal-state comparisons, and
  10-second deadlines with kill/reap cleanup.
- Exercise normal exit, raw-mode Ctrl-C, pre-terminal failure, active handled
  error, and recoverable panic restoration through native Linux PTYs.
- Delay handled-error and panic diagnostics until terminal cleanup completes.

Selection:

- Changed paths: Cargo dependencies/lockfile, process and terminal boundaries,
  CLI/PTY tests, generated test manifests and validator, CI, test catalog,
  README, implementation/commit trackers, and test report.
- Classified areas: CLI/startup, terminal lifecycle, tests/fixtures, CI,
  dependency graph, documentation, and test governance.
- Selected case IDs: `QG-001` through `QG-005`, `QG-007` through `QG-014`,
  `CLI-001`, `CLI-002`, `CLI-004`, `CLI-005`, `CLI-010`, `TERM-001` through
  `TERM-005`, `TERM-008`, `KEY-004`, `HELP-002`, `HELP-003`, `ERR-002`,
  `PROP-010`, `SUP-001` through `SUP-004`, and `SUP-006` through `SUP-008`.
- Profiles run: registry freshness, `pr-core`, Linux `native-pty`, and dependency
  policy. Planned render, security-target, scheduled, weekly, and release target
  commands did not run because their feature implementations do not exist yet.
- Fixtures: no book fixture was opened. `tests/fixtures.toml` records planned
  assets and the existing ignored Gutenberg provenance/hashes.
- Environment: local `ENV-LINUX-PTY` equivalent on CachyOS rather than the
  required Ubuntu 24.04 CI row; no macOS or Windows environment was claimed.

Checks:

| Command or procedure | Result |
| --- | --- |
| `python3 tools/case_registry.py check` | Passed: 332 unique IDs, no unknown/orphan locations, profiles and six cumulative gates agree |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 15 library tests, 3 CLI tests, and 5 Linux PTY tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed: advisories, bans, licenses, and sources after dev-dependency additions |
| `git diff --check` | Passed |
| `cargo clean` | Passed: removed 1,886 files and 409.9 MiB |

Blocked or unavailable Phase 0 evidence:

- `TERM-011` remains Blocked, owned by Phase 0. `DEC-TEST-013` has not selected
  supported process signals. Raw-mode Ctrl-C key input is compensating evidence;
  removal requires a resolved signal/checkpoint policy and native PTY matrix.
  Review date: September 20, 2026.
- `TERM-012` remains Blocked, owned by Phase 0. Linux tests prove restoration
  from the ordinary baseline, but Crossterm cannot query every preexisting ANSI
  cursor/screen/paste mode. Removal requires an exact capture or documented
  ownership policy and tests for each observable initial state. Review date:
  September 20, 2026.
- Rust 1.88, Ubuntu 24.04, macOS 15, and Windows Server 2025 hosted jobs did not
  run locally. The fixed CI rows and Unix PTY jobs are definitions, not evidence.
- Windows ConPTY lifecycle, externally delivered signals, SSH/tmux, and named
  GUI terminal rows did not run and support is not claimed.

### Start the Rust foundation

**Commit subject:** `feat: start Rust foundation`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:15 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64; rustc/cargo 1.97.1;
cargo-deny 0.20.2

Scope:

- Initialize the locked Rust package, application loop, terminal guard, base
  Ratatui shell, CLI, CI definition, and dependency policy.
- Exercise setup rollback, normal cleanup, cleanup after one restoration error,
  unwinding cleanup, state/focus transitions, deterministic rendering, and
  pre-terminal CLI behavior.

Selection:

- Changed paths: Cargo package and lockfile, application/CLI/terminal/UI source,
  tests, CI, dependency policy, README, plan, trackers, and test report.
- Classified areas: CLI/startup, terminal lifecycle, theme/UI foundation,
  dependency/feature flags, CI, license, and documentation.
- Selected case IDs: `QG-001` through `QG-005`, `QG-007` through `QG-013`,
  `CLI-001`, `CLI-002`, `CLI-005`, `TERM-001`, `TERM-003` through `TERM-005`,
  `TERM-007`, `PROP-010`, `SUP-001` through `SUP-004`, and `SUP-006` through
  `SUP-008`.
- Profiles run: the currently materialized local `pr-core` commands and
  dependency-policy check. The frozen machine-readable manifests do not exist
  yet and are not claimed complete.
- Fixtures: no book fixtures or downloaded corpus files were used.
- Environment: local non-PTY Linux process tests and Ratatui `TestBackend` at
  `40x10`; no native terminal compatibility row was claimed.

Checks:

| Command or procedure | Result |
| --- | --- |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `cargo test --locked` | Passed: 10 unit/render tests and 3 CLI process tests |
| `cargo test --doc --locked` | Passed: 0 doctests present |
| `/tmp/opencode/cargo-deny/bin/cargo-deny check` | Passed: advisories, bans, licenses, and sources |
| `cargo clean` | Passed: removed 1,444 files and 324.8 MiB |

Skipped or incomplete evidence:

- Rust 1.88 validation did not run because only Rust 1.97.1 is installed and
  `rustup` is unavailable in the local environment. A separate locked Rust 1.88
  CI job is defined but has not produced hosted evidence yet.
- Native PTY, Ctrl-C/signal, initial terminal-state capture, panic-diagnostic,
  SSH/tmux, macOS, and Windows restoration cases did not run. Current terminal
  case tests prove the guard logic with an injected control boundary only, so
  those case IDs are not promoted to Passing.
- The machine-readable case registry, executable profile and phase-gate
  manifests, fixture manifests, and full hermetic harness remain Phase 0 work.
- Hosted CI was defined but did not run locally. `SUP-004` has static workflow
  evidence only; release trigger and artifact controls remain later work.

### Define quality, testing, and UI implementation standards

**Commit subject:** `docs: define implementation standards`

**Revision:** This commit

**Recorded:** August 20, 2026 at 12:05 AM EDT

**Environment:** Linux 7.1.8-1-cachyos x86-64

Scope:

- Establish enforceable Rust implementation and review standards.
- Add a concrete catalog of stable test IDs, profiles, fixtures, environments,
  phase gates, selection rules, and completion criteria.
- Connect both standards to project planning and implementation tracking.
- Define exceptions, unresolved test decisions, and validation expectations.
- Add detailed ASCII UI mockups and implementation guidance.
- Reconcile the implementation roadmap and tracker while retaining six phases.

Selection:

- Changed paths: project standards, test catalog, UI specification, roadmap,
  tracker, README, decision log, and test report.
- Classified areas: documentation, test governance, UI hierarchy, phase gates,
  accessibility, and release evidence.
- Selected catalog IDs: `QG-010` for report completeness; no Rust implementation
  case was executable because the Cargo package and machine registry do not yet
  exist.
- Profiles: documentation validation only. Rust, render, PTY, security, fuzz,
  benchmark, native, and release profiles remain Planned.
- Fixtures and environments: no book fixture was opened; local Linux environment
  was used only for deterministic document validation.
- Blocked cases: the named `DEC-TEST-*` rows remain owned by their target phases
  with removal conditions stated in `testcases.md`; none was represented as
  passing.

Review passes:

| Pass | Focus | Incorporated results |
| --- | --- | --- |
| 1 | Locked feature coverage | Added default paged geometry, both-mode jumps, EPUB semantics, integrated images, ordered fallbacks, restore journeys, durable annotations, literal search, and exact theme/status/help requirements. |
| 2 | Security, privacy, and hostile input | Added exact boundary method, corrected atomic-save oracle, SVGZ/XML depth, hostile state paths, URL launch hardening, no-log inventory, filesystem/privacy canaries, decoder and supply-chain approval. |
| 3 | Terminal, platform, accessibility, and release | Added named environment rows, signal/initial-state restoration, native keyboard cases, real image-protocol cleanup, terminal Unicode/bidi limits, assistive technology matrix, benchmark method, and native release evidence. |
| 4 | Framework traceability and governance | Added case ownership/status lifecycle, machine registry, executable profile manifests, hermetic harness, fixture manifests, exact selection/report schema, immutable IDs, regression governance, and cumulative phase gates. |

Synchronization reviews:

| Review | Result |
| --- | --- |
| UI contract review | Added missing link focus, text selection, point/range note flow, search history, theme selection, status glossary, long-value inspection, mode-safe tiny-terminal recovery, confirmations, image anchor compensation, and explicit open UI decisions. |
| Six-phase roadmap review | Confirmed six phases remain correct; assigned registry/harness/UI shell to Phase 0 and strengthened the existing Phase 1-5 work and cumulative exit evidence. |

Checks:

| Check | Result |
| --- | --- |
| Standards cover correctness, security, privacy, testing, and maintainability | Passed |
| Required validation agrees with the project plan | Passed |
| Cargo cleanup and per-commit reporting requirements are explicit | Passed |
| Four independent completeness reviews completed and incorporated | Passed |
| Stable test case ID uniqueness | Passed: 332 definitions, no duplicates or dangling exact references |
| Markdown table and code-fence structure | Passed across eight files |
| Local Markdown links and table-of-contents entries | Passed across eight files |
| UI file contains ASCII only | Passed |
| Markdown files contain no em dash characters | Passed |
| Required Cargo command list agrees across standards and project plan | Passed |
| Six-phase count and numbering agree across plan, tracker, tests, and UI | Passed: `0,1,2,3,4,5` in each |
| `git diff --check` | Passed |
| Rust tests | Not applicable: the Cargo package does not exist yet |
| `cargo clean` | Not applicable: no Cargo command ran and no `target/` directory was created |

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
