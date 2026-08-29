# TermLeaf Test Cases

**Last updated:** August 20, 2026 at 1:02 AM EDT

## Table of Contents

- [Purpose](#purpose)
- [Sources of Truth](#sources-of-truth)
- [Case Model](#case-model)
- [Status and Priority](#status-and-priority)
- [Case Registry and Governance](#case-registry-and-governance)
- [Test Layers](#test-layers)
- [Execution Profiles](#execution-profiles)
- [Hermetic Harness](#hermetic-harness)
- [Tools and Test Layout](#tools-and-test-layout)
- [Fixture Registry](#fixture-registry)
- [Environment Matrix](#environment-matrix)
- [Open Test Decisions](#open-test-decisions)
- [Boundary Method](#boundary-method)
- [Change-to-Suite Matrix](#change-to-suite-matrix)
- [Quality Gates](#quality-gates)
- [Foundation and CLI Cases](#foundation-and-cli-cases)
- [Terminal Lifecycle Cases](#terminal-lifecycle-cases)
- [Native Keyboard Cases](#native-keyboard-cases)
- [Plain-Text Cases](#plain-text-cases)
- [Document Model and Markdown Cases](#document-model-and-markdown-cases)
- [EPUB and Archive Cases](#epub-and-archive-cases)
- [Image Cases](#image-cases)
- [Layout and Unicode Cases](#layout-and-unicode-cases)
- [Navigation and Reading Mode Cases](#navigation-and-reading-mode-cases)
- [Search Cases](#search-cases)
- [Configuration and Persistence Cases](#configuration-and-persistence-cases)
- [Recent Book Cases](#recent-book-cases)
- [Bookmark and Annotation Cases](#bookmark-and-annotation-cases)
- [Theme and Rendering Cases](#theme-and-rendering-cases)
- [Status, Help, and Error Cases](#status-help-and-error-cases)
- [UI Interaction Cases](#ui-interaction-cases)
- [Link, Privacy, and Data Cases](#link-privacy-and-data-cases)
- [Concurrency and Cancellation Cases](#concurrency-and-cancellation-cases)
- [Accessibility Cases](#accessibility-cases)
- [Performance Cases](#performance-cases)
- [Benchmark Method](#benchmark-method)
- [Supply Chain and Release Cases](#supply-chain-and-release-cases)
- [Property Test Catalog](#property-test-catalog)
- [Fuzz Target Catalog](#fuzz-target-catalog)
- [Phase Gates](#phase-gates)
- [Defect and Regression Process](#defect-and-regression-process)
- [Per-Commit Selection](#per-commit-selection)
- [Completion Rules](#completion-rules)

## Purpose

This document is the concrete test framework for TermLeaf. It names the cases
that prove the first-release contract, the layer where each case belongs, the
fixtures and environments it needs, and the evidence required to pass.

The catalog is intentionally implementation-aware but not implementation-bound.
Stable IDs survive test file moves and allow a commit, defect, benchmark, phase
gate, or release report to identify exactly what ran. The complete catalog is
materialized in `tests/case_registry.toml`. Cases remain **Planned** until
reviewed evidence changes their status; package existence or a partial test
alone does not make a case Passing. A case becomes **Passing** only after an
automated test or recorded manual procedure exists and passes in its required
environment.

## Sources of Truth

Resolve disagreement in this order:

1. Reader-visible behavior and limits in `project_plan.md`.
2. Engineering and validation rules in `code_quality.md`.
3. Current delivery state in `implementation_tracker.md`.
4. UI hierarchy and interaction intent in `ui_mockups.md`.
5. This catalog's test mapping and procedures.
6. Per-commit evidence in `testreport.md`.

Changing a promised behavior or safety limit requires updating the source plan
and affected test cases together. A passing test that asserts obsolete behavior
is a defect, not evidence.

## Case Model

Every implemented case must carry this information in code, fixture metadata,
or the test report:

| Field | Required content |
| --- | --- |
| ID | Stable area prefix and three digits, such as `TXT-001` |
| Requirement | Behavior, limit, risk, or regression being proved |
| Priority | `P0`, `P1`, or `P2` |
| Layer | Unit, property, render, integration, PTY, fuzz, benchmark, or manual |
| Preconditions | State, platform, terminal capability, and fixture |
| Action | Input or user journey performed by the test |
| Oracle | Directly asserted result and forbidden side effects |
| Profile | Smallest execution profile that must include the case |
| Status | Planned, Implemented, Passing, Blocked, or Retired |

Rust test names should include the lowercase ID and behavior:

```rust
#[test]
fn txt_001_accepts_valid_utf8_without_bom() {
    // Arrange, act, and assert through readable code.
}
```

Parameterized boundary tests may implement several IDs or one ID with named
cases. Failure output must print the ID, fixture, relevant limit, and observed
value. Snapshot names must include the case ID.

## Status and Priority

| Status | Meaning |
| --- | --- |
| Planned | Requirement is cataloged but no executable proof exists. |
| Implemented | Test exists but has not passed every required environment. |
| Passing | Required assertions and environments passed at the recorded revision. |
| Blocked | A named dependency, fixture, platform, or product decision prevents execution. |
| Retired | Requirement was removed or replaced, with the reason recorded. |

| Priority | Meaning | Merge policy |
| --- | --- | --- |
| P0 | Data safety, hostile input, terminal restoration, or core reading path | Must pass whenever applicable; release blocker |
| P1 | Promised first-release behavior or major compatibility path | Must pass before its phase gate and release |
| P2 | Secondary presentation, diagnostics, or non-blocking compatibility depth | Must pass before claiming the affected behavior |

Priority merge policy applies to required behavioral cases. A `FUZZ-*` row's
priority describes discovery triage only and never makes that optional target a
merge, security-profile, phase-gate, or release requirement.

No failing case may be hidden by changing it to Blocked. A Blocked case needs an
owner, reason, compensating evidence, and removal condition in `testreport.md`.

## Case Registry and Governance

This document is the authoritative behavioral catalog. The generated
`tests/case_registry.toml` is its machine-readable index, and
`tests/case_registry.overrides.toml` records reviewed status, implementation,
and evidence data that cannot be derived from prose. The Phase Gates section
owns each case unless a row says otherwise. The registry contains these fields:

| Field | Rule |
| --- | --- |
| `id` | Matches `[A-Z][A-Z0-9]*-[0-9]{3}` and is never reused, including after retirement |
| `title` | One stable behavioral description |
| `status` | Planned, Implemented, Passing, Blocked, or Retired |
| `owner` | Delivery phase plus current responsible contributor or team |
| `implements` | Product requirement and related behavioral case IDs |
| `location` | Rust test, fuzz target, benchmark, or manual procedure path |
| `profiles` | Exact executable suite manifests containing the case |
| `environments` | Named required environment IDs |
| `fixtures` | Registered fixture IDs and hashes or generator versions |
| `last_evidence` | Revision and report or CI artifact reference |

CI must reject duplicate, malformed, unknown, and orphaned IDs. Every executable
test, manual procedure, benchmark, and fuzz target must map back to at least one
catalog ID, and every Implemented or Passing ID must map to executable evidence.
Property and fuzz cases record `implements` links to the behavioral cases they
support. `FUZZ-*` IDs are optional discovery metadata: they may appear only as
optional `weekly` cases, never as required `security` or `phase-gate-N` members.
Their deterministic linked behavior remains required under its ordinary IDs and
profiles. Retired IDs remain reserved and name their replacement when one exists.

Status changes follow these rules:

- Planned to Implemented requires an executable test or reviewed manual
  procedure and registered fixtures.
- Implemented to Passing requires every named profile and environment to pass.
- Any status to Blocked requires an owner, reason, compensating evidence,
  removal condition, and review date.
- Passing returns to Implemented when its requirement, implementation, fixture,
  or required environment changes materially.
- Retired requires the corresponding product decision and must not hide a known
  defect.

## Test Layers

| Layer | Scope | Rules |
| --- | --- | --- |
| Unit | One module or pure operation | No terminal, network, user directories, or wall-clock dependence |
| Property | Invariants over generated values | Fixed seed on failure and minimal reproducible input retained |
| Render | Application cell grid or Ratatui test backend | Direct cell and width assertions accompany reviewed snapshots |
| Integration | Filesystem, parser stack, configuration, or process boundary | Isolated temporary directories and deterministic environment |
| PTY | Real process and terminal lifecycle | Strict timeout, child cleanup, fixed locale and `TERM`, low parallelism |
| Fuzz | Optional coverage-guided discovery at untrusted boundaries | Bounded harness, seed corpus, crash artifact retention; never substitutes for deterministic evidence |
| Benchmark | Responsiveness and memory budgets | Release profile, recorded hardware, stable fixtures, no shared-runner gate |
| Manual | Hardware, terminal, assistive technology, or packaging behavior | Written steps, expected result, tester, platform, and date |

`Tool` and `Review` are evidence methods, not test layers. `CI`, `Scheduled`, and
`Native` are execution environments. Combined rows name one primary layer and
list additional evidence in the case registry rather than inventing new layer
values.

Tests should run at the lowest layer that can prove the behavior. P0 behavior
crossing process or terminal boundaries also needs evidence at that boundary.

## Execution Profiles

| Profile | Trigger | Executable manifest |
| --- | --- | --- |
| `pr-core` | Every Rust change | `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --locked`; `cargo test --doc --locked`; `cargo deny check` |
| `pr-render` | UI, layout, theme, status, help, or terminal cell changes | `pr-core`; render integration target; snapshot review with direct assertions |
| `native-pty` | Terminal lifecycle, input, signal, or release changes | `pr-core`; serial PTY integration target with named `ENV-*` rows and 30-second per-case timeout |
| `security` | Parser, archive, image, URL, path, state, dependency, worker, or allocation changes | `pr-core`; security integration target; deterministic boundary, malformed-input, hostile-fixture, property, and bounded-mutation cases; side-effect inventory |
| `scheduled` | Nightly | `pr-core`; all properties at extended case count; local licensed corpus; leak and retained-memory checks |
| `weekly` | Optional weekly discovery | Explicitly selected fuzz targets with run-specific budgets; dependency graph review; selected release-profile benchmarks |
| `phase-gate-N` | Completion of phase N | Frozen exact ID and environment manifest for N plus every earlier phase gate and all permanent regressions |
| `release` | Release candidate and protected tag | Every prior gate; required `ENV-*` rows; supply chain, package, install, upgrade where applicable, notices, and checksums |

Every profile records exact commands, revision, operating system, toolchain,
features, fixtures, seeds, and skipped cases in `testreport.md`. Local Rust
profiles finish with `cargo clean` after all Cargo commands complete.

Phase 0 must materialize these profiles as versioned scripts or CI jobs. Each
manifest records exact case IDs, commands, features, runner, timeout, retry
policy, parallelism, and retained artifacts. Retries may diagnose infrastructure
failure but may not turn a failed deterministic test into a pass. Multiple
changed areas select the union of their manifests; exclusions require an ID and
reason in the test report.

## Hermetic Harness

All automated cases use one shared harness contract:

- Redirect `HOME`, XDG config/data/state/cache variables, Windows AppData, and
  platform temp paths into one case-owned temporary root.
- Deny network access and record attempted sockets, DNS, process launches, and
  filesystem operations for privacy or parser cases.
- Start from a minimal environment allowlist. Record every extra variable needed
  by a native test.
- Inject a fixed clock and time zone for status, recency, message-duration, and
  persistence assertions.
- Fix locale, `TERM`, viewport, color capability, image capability, and ambiguous
  width policy unless the case varies that dimension.
- Use deterministic seeds and print the seed on generated failure.
- Give ordinary integration cases a 10-second timeout and PTY cases a 30-second
  timeout unless the registry justifies another value.
- Run PTY, signal, global-environment, and filesystem-fault cases in named serial
  groups; other cases must remain parallel-safe.
- Track child processes and worker threads. Teardown fails if any remain, if the
  terminal differs from its captured initial state, or if unregistered files
  remain outside the temporary root.
- Simulate permissions, short writes, full destinations, replacement races, and
  clock progress through injectable boundaries rather than host-dependent luck.
- Never contact Project Gutenberg, metadata services, browsers, or update
  services during a test run.

Flaky tests remain failures. A temporary quarantine uses Blocked status, names
an owner and expiry, and retains the original failure evidence.

## Tools and Test Layout

The planned layout may change with the package structure, but case ownership
must remain visible:

```text
src/**                    colocated pure unit tests
tests/model/**            cross-module document and reader tests
tests/render/**           cell assertions and reviewed snapshots
tests/integration/**      filesystem, configuration, and parser-stack tests
tests/pty/**              process and terminal lifecycle journeys
tests/fixtures/**         small generated or licensed fixtures plus provenance
fuzz/fuzz_targets/**      untrusted-input harnesses
benches/**                Criterion benchmarks
```

| Need | Planned tool |
| --- | --- |
| Unit and integration | Rust built-in test harness |
| Generated invariants | `proptest` |
| Reviewed render output | `insta` plus direct assertions |
| ANSI terminal model | `vt100` |
| Native process journeys | `portable-pty` |
| Parser robustness | `cargo-fuzz` and libFuzzer |
| Performance trends | Criterion and platform memory measurement |
| Dependency policy | `cargo-deny`, `cargo tree`, and RustSec data |

Tests must not require network access. Downloading or refreshing external
fixtures is a separate, explicit maintenance action.

## Fixture Registry

Small committed fixtures must include source, license, creation method, and the
cases they serve. Generated hostile files should come from deterministic fixture
builders where practical so large payloads do not bloat the repository.

Each fixture receives a machine-readable manifest containing ID, repository
path or generator, exact source URL when external, retrieval date, SPDX/license
conclusion, SHA-256, byte size, generator revision and parameters, expected
properties, and served case IDs. CI verifies committed hashes and rejects
unregistered fixture references. Decoder fixtures are registered per format,
not hidden behind one generic raster entry. The ignored Gutenberg IDs map to the
exact filenames and hashes recorded in `testreport.md`.

| Fixture ID | Content | Storage | Primary use |
| --- | --- | --- | --- |
| `FX-TXT-UTF8` | ASCII, accents, combining marks, CJK, emoji, and blank lines | Committed small fixture | Decoding, layout, search |
| `FX-TXT-UTF16LE` | BOM-marked UTF-16 little-endian text | Committed small fixture | Decoding |
| `FX-TXT-UTF16BE` | BOM-marked UTF-16 big-endian text | Committed small fixture | Decoding |
| `FX-TXT-BAD` | Invalid UTF-8 and unmarked UTF-16 samples | Committed generated fixture | Error handling |
| `FX-MD-SEMANTIC` | Headings, lists, quotes, table, links, image, code, and raw HTML | Committed authored fixture | Markdown conversion |
| `FX-MD-CODE` | Inline code, fenced languages, tabs, long lines, and terminal transcript | Committed authored fixture | Programming-book behavior |
| `FX-EPUB2` | Minimal licensed or generated EPUB 2 with NCX | Committed small fixture | EPUB 2 semantics |
| `FX-EPUB3` | Minimal licensed or generated EPUB 3 with nav document | Committed small fixture | EPUB 3 semantics |
| `FX-EPUB-MALFORMED` | Recoverable XHTML and broken semantic references | Committed generated fixture | Recovery and diagnostics |
| `FX-EPUB-HOSTILE` | Parameterized unsafe paths, sizes, ratios, overlap, and encryption | Generated during tests | Archive security |
| `FX-IMG-RASTER` | One minimal sample per safely enabled raster decoder | Committed generated or licensed fixtures | Image decoding |
| `FX-IMG-SVG` | Safe SVG, SVGZ, and blocked external-resource variants | Committed generated fixtures | SVG safety |
| `FX-STATE` | Current, old, future, corrupt, and interrupted state files | Committed generated fixtures | Persistence |
| `FX-GUT-11` | Gutenberg 11 EPUB 3 with images and TXT | Ignored local download | Realistic illustrated journey |
| `FX-GUT-84` | Gutenberg 84 legacy EPUB and TXT | Ignored local download | Long chapter sequence |
| `FX-GUT-1342` | Gutenberg 1342 legacy EPUB and TXT | Ignored local download | Larger prose journey |

Full Gutenberg files support local and scheduled corpus runs. CI must not depend
on ignored files; equivalent core assertions use committed small fixtures.

## Environment Matrix

Automated tests parameterize deterministic dimensions and capabilities. Native
claims require the real environments below before release.

| Dimension | Required values |
| --- | --- |
| Operating system | Linux, macOS, Windows |
| Terminal | GNOME Terminal, Konsole, system macOS Terminal, Windows Terminal, Kitty or WezTerm |
| Session | Direct, SSH, and tmux where practical |
| Viewport | `120x40`, `80x24`, `40x10`, minimum supported, and below minimum |
| Color | True color, 256 color, 16 color or terminal default, and `NO_COLOR` |
| Image capability | Kitty, Sixel, iTerm2, true-color cells, and caption only |
| Locale | UTF-8 English, one CJK locale, and deterministic `C` where suitable |
| Input | Conventional keys, Vim-style keys, paste, resize, focus, Ctrl-C |
| Filesystem | Read-only source, missing source, denied permission, moved source, full or failed state destination |

Unsupported combinations must fail or fall back explicitly. A cross-compiled
binary does not satisfy a native behavior case.

The release matrix is a set of named rows, not a Cartesian product. Stage 0 must
replace every `TBD` with a tested version and mark each row Required,
Informational, or Deferred before the corresponding platform can be claimed.

| Environment ID | Candidate tuple | Initial gate state |
| --- | --- | --- |
| `ENV-LINUX-PTY` | Ubuntu 24.04 x86-64; kernel PTY; `TERM=xterm-256color` | Required for Phase 0 lifecycle evidence |
| `ENV-MAC-PTY` | macOS 15 arm64; native PTY; `TERM=xterm-256color` | Required for Phase 0 lifecycle evidence |
| `ENV-WIN-PTY` | Windows Server 2025 x86-64; ConPTY | Required for Phase 0 lifecycle evidence |
| `ENV-LINUX-GNOME` | Supported Linux and GNOME Terminal version unselected; direct session | Deferred; no compatibility claim |
| `ENV-LINUX-KONSOLE` | Supported Linux and Konsole version unselected; direct session | Deferred; no compatibility claim |
| `ENV-LINUX-MODERN` | Supported Linux and Kitty or WezTerm version unselected; direct, SSH, and tmux rows | Deferred to terminal/version decision |
| `ENV-MAC-TERM` | Supported macOS and system Terminal version unselected | Deferred; no compatibility claim |
| `ENV-MAC-ITERM` | Supported macOS and iTerm2 version unselected; image-capability row | Deferred to image protocol decision |
| `ENV-WIN-WT` | Supported Windows and Windows Terminal version unselected | Deferred; no compatibility claim |
| `ENV-SIXEL` | Native OS, terminal, and Sixel implementation unselected | Deferred until an implementation is selected |

Every finalized row records artifact type, terminal version, session nesting,
`TERM`, locale, font for Unicode manual checks, color/image capability, required
case IDs, and evidence owner. `REL-006` passes only when every Required row
passes. Waivers need an owner, reason, compensating evidence, and removal
condition. Optional rows cannot support a public compatibility claim.

## Open Test Decisions

A case that depends on one of these choices remains Blocked rather than allowing
multiple outcomes. Resolving a row requires updating `project_plan.md`, the
affected exact oracle, and the case registry together.

| Decision ID | Blocks | Required decision |
| --- | --- | --- |
| `DEC-TEST-002` | `CLI-009`, `A11Y-005` | Exact piped-input, piped-output, and plain-text/noninteractive behavior |
| `DEC-TEST-003` | `SEC-008` | Compression-ratio formula, inclusivity, per-entry/aggregate scope, zero-byte handling, and small-file threshold |
| `DEC-TEST-004` | `SEARCH-007` | Empty/control query behavior and maximum query length |
| `DEC-TEST-005` | `CFG-003` | Unknown-key warning policy and invalid-config startup behavior |
| `DEC-TEST-006` | `STATE-007`, `ANN-008` | Exact document identity and relocation evidence for moved or edited books |
| `DEC-TEST-007` | `STATE-009` | Concurrent writer locking, merge, or last-writer policy |
| `DEC-TEST-008` | `LINK-001` | Exact external scheme allowlist and URL length policy |
| `DEC-TEST-009` | `HELP-002` | Searchable help, section-scannable help, or both |
| `DEC-TEST-010` | `NAV-007`, native `KEY` cases | Final conflict-free action-to-key map and multikey-prefix policy |
| `DEC-TEST-011` | `STATUS-002`, `STATUS-004`, `STATUS-005` | Exact field collapse order, location/percentage/page formulas, rounding, clock format, and message lifetime |
| `DEC-TEST-012` | config/state/input limits | Numeric limits for TXT, Markdown, config, state, queries, notes, URLs, recents, annotations, and total persisted state |
| `DEC-TEST-015` | `EPUB-003` | Exact EPUB manifest fallback selection and failure rule |
| `DEC-TEST-016` | `SEC-007` | Whether an over-limit XHTML chapter rejects the book or becomes a bounded skipped chapter |
| `DEC-TEST-017` | `ANN-006`, temporary-view returns | Exact return-stack behavior after jumping away from a prior passage |

`DEC-TEST-013` was resolved by `DD-018`: Phase 0 supports raw-mode Ctrl-C on all
targets and catchable external `SIGINT` on POSIX. Windows console Ctrl-C,
Ctrl-Break, close, logoff, and shutdown events, POSIX termination/hangup, and
uncatchable events are not claimed until a safe native harness and checkpoint
semantics require them.

`DEC-TEST-001` was resolved by `DD-024`: detection is extension-first and
case-insensitive. Phase 1 ships exactly one adapter, so only `.txt` opens;
Markdown and EPUB extend the accepted-extension table in their own delivery
phases instead of sniffing content ahead of their parsers. Content validity is
still enforced after the extension gate, so a `.txt` file holding binary data
fails decoding with a typed reason.

`DEC-TEST-014` is resolved by `DD-031`: an explicit image override wins when
capability evidence is positive or absent, but explicit negative evidence
returns one typed incompatibility error instead of silently selecting another
backend. Automatic selection requires positive evidence.

`DEC-TEST-018` is resolved by `DD-032`: two ordinary worker threads share a
nonblocking queue of eight waiting requests and a 64 MiB queued/running input
budget. Over-capacity submissions reject immediately with a typed reason;
generation rollover cancels queued work and discards stale completions.

## Boundary Method

Every numeric policy uses one shared vector unless a case records why a point is
meaningless:

```text
0 or minimum valid value
limit - 1
limit
limit + 1
maximum representable and checked-arithmetic overflow inputs
dishonest metadata below the limit with actual work above it
```

Limits are binary MiB where the plan says MiB. Tests name whether a value is
compressed bytes, actual decompressed bytes, XML nodes, dimensions, pixels,
entries, allocations, queue slots, or logical text units. Exact-limit acceptance
or rejection is part of the product policy and cannot be inferred from “around.”

The boundary harness independently varies one limit while holding others valid,
then combines near-limit values to catch aggregate bypasses. It asserts the
typed result, allocation high-water mark, elapsed-work bound, preserved prior
state, and absence of network, process, host-file, or extraction side effects.

## Change-to-Suite Matrix

Every code change selects at least the mapped case families. Reviewers add
cross-cutting profiles when behavior crosses boundaries.

| Changed area | Required case families |
| --- | --- |
| CLI or startup | `CLI`, `TERM`, `ERR`, `PRIV` |
| Plain-text ingestion | `TXT`, `MODEL`, `LAY`, `SEARCH`, `SEC` |
| Markdown ingestion | `MD`, `MODEL`, `LAY`, `LINK`, `SEC` |
| EPUB archive or parser | `EPUB`, `SEC`, `MODEL`, `LINK`, `IMG`; optionally select linked `FUZZ` discovery |
| Image pipeline | `IMG`, `SEC`, `CON`, `RENDER`, performance |
| Document model | `MODEL`, every active format adapter, `LAY`, `SEARCH`, `ANN` |
| Layout or Unicode | `LAY`, `NAV`, `SEARCH`, `RENDER`, `A11Y`, performance |
| Navigation or reading mode | `NAV`, `LAY`, `STATE`, `STATUS`, PTY journey |
| Search | `SEARCH`, `LAY`, `STATE`, `PRIV` |
| Configuration or state | `CFG`, `STATE`, `RECENT`, `ANN`, `PRIV`; optionally select linked `FUZZ` discovery |
| Recent books | `RECENT`, `STATE`, `PRIV` |
| Bookmarks or annotations | `ANN`, `STATE`, `LAY`, `A11Y` |
| Theme or UI | `UI`, `RENDER`, `THEME`, `STATUS`, `HELP`, `A11Y` |
| External links | `LINK`, `PRIV`, native launch smoke cases |
| Worker threads or channels | `CON`, affected parser or image cases, shutdown PTY |
| Dependency or feature flags | `QG`, `SUP`, all-feature build, affected platform smoke cases |
| Packaging or release | `REL`, `SUP`, `native-pty`, installation and upgrade cases |
| Tests or fixtures | Cases named by the changed test plus `QG`, fixture-manifest validation, and every consumer ID |
| Fuzz or benchmark harness | Related `FUZZ` or `PERF` ID, behavioral `implements` IDs, `QG`, and smoke execution |
| Build script or CI workflow | `QG`, `SUP`, profile-manifest validation, and one clean runner execution |
| Documentation or test catalog | Link/anchor validation, ID-registry validation, affected requirement IDs, and `QG-010` |
| License, notice, or non-code asset | `SUP`, provenance manifest, package-content, and notice generation cases |

The matrix selects candidate families, then the machine-readable registry
expands them into exact IDs and executable manifests. A commit report must list
changed paths, classified areas, selected exact IDs, selected profiles and
commands, required environments, and every excluded mapped ID with its reason.

## Quality Gates

| ID | Priority | Case and oracle | Layer/profile |
| --- | --- | --- | --- |
| `QG-001` | P0 | `cargo fmt --check` exits successfully with no generated diff. | Tool / `pr-core` |
| `QG-002` | P0 | `cargo clippy --all-targets --all-features -- -D warnings` passes; suppressions are narrow and justified. | Tool / `pr-core` |
| `QG-003` | P0 | `cargo test --locked` passes with no ignored failure introduced by the change. | Tool / `pr-core` |
| `QG-004` | P1 | `cargo test --doc --locked` compiles and passes every doctest. | Tool / `pr-core` |
| `QG-005` | P0 | `cargo deny check` passes advisories, licenses, sources, and bans policy. | Tool / `security` |
| `QG-006` | P1 | Minimum supported Rust builds the locked default feature set. | Integration / `phase-gate` |
| `QG-007` | P1 | Current stable Rust builds and tests all targets and features. | Integration / `pr-core` |
| `QG-008` | P0 | TermLeaf crates contain no unsafe code and the manifest lint forbids it. | Static / `pr-core` |
| `QG-009` | P1 | No production placeholder panic, debug print, broad lint allowance, or untracked `TODO` remains. | Review / `pr-core` |
| `QG-010` | P1 | The test report names every run, skip, fixture, environment, and final `cargo clean` outcome honestly. | Review / every commit |
| `QG-011` | P0 | Inspect `unwrap`, `expect`, `panic!`, `todo!`, and `unreachable!`; none is user-reachable, while narrow internal-invariant uses name the invariant and have surrounding tests. | Static/review / `pr-core` |
| `QG-012` | P0 | Check the crate graph; document, layout, reader, and persistence have no Ratatui/Crossterm dependency or hidden UI callback cycle. | Static / `pr-core` |
| `QG-013` | P1 | Inspect mutable globals and public domain fields; no global mutable state exists and public mutation cannot construct invalid state. | Static/review / `pr-core` |
| `QG-014` | P1 | Validate registry against tests, profiles, fixtures, and evidence; no ID is malformed, unknown, orphaned, duplicated, or illegally reused. | Tool / `pr-core` |

## Foundation and CLI Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `CLI-001` | P1 | Run with `--help`. | Usage, supported path argument, and current options print without terminal initialization. | Integration / `pr-core` |
| `CLI-002` | P1 | Run with `--version`. | Exact package version prints and process exits successfully. | Integration / `pr-core` |
| `CLI-003` | P0 | Pass an existing supported local path. | One immutable source is opened and no unrelated path is inspected. | Integration / `pr-core` |
| `CLI-004` | P1 | Start without a path. | Recent-books screen opens without directory scanning. | PTY / `native-pty` |
| `CLI-005` | P1 | Pass a missing path. | Error names the safe path, says it is missing, suggests recovery, and terminal remains normal. | Integration / `pr-core` |
| `CLI-006` | P1 | Pass an unreadable path. | Permission error is specific; source and state remain unchanged. | Integration / `pr-core` |
| `CLI-007` | P1 | Pass an unsupported extension and misleading extension/content pair. | Extension-first detection per the resolved `DEC-TEST-001` (DD-024): a case-insensitive `.txt` extension opens, any other or missing extension returns one typed unsupported-format error before terminal setup, and `.txt` content still decodes strictly. | Unit / `pr-core` |
| `CLI-008` | P1 | Supply config plus explicit CLI overrides. | Only explicitly supplied CLI values override config; defaults remain lowest precedence. | Unit / `pr-core` |
| `CLI-009` | P1 | Pipe input or output instead of a TTY. | Remains Blocked until `DEC-TEST-002`; final behavior emits no unapproved full-screen control sequences. | PTY / `native-pty` |
| `CLI-010` | P0 | Force failure before and after terminal initialization. | Pre-init failure emits no cleanup sequences; post-init failure restores every changed mode. | PTY / `native-pty` |
| `APP-001` | P0 | Enumerate every first-release view identity. | Each view derives exactly one exclusive focus owner; text-entry, list, reader, confirmation, error, and suspended focus cannot coexist. | Unit / `pr-core` |
| `APP-002` | P0 | Exercise foundation quit, help, and return actions. | The state loop stops cleanly, help uses the shared binding registry, and return restores the invoking view and focus. | Unit / `pr-core` |
| `APP-003` | P1 | Render the foundation recent-books shell twice at `40x10`. | Required title, empty state, and action band occupy deterministic cells with no stale output. | Render / `pr-render` |
| `APP-004` | P0 | Inject one restoration failure after complete setup. | Every remaining cleanup operation is attempted once and the first cleanup error remains primary. | Unit / `pr-core` |

## Terminal Lifecycle Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `TERM-001` | P0 | Launch and quit normally. | Raw mode, cursor, alternate screen, mouse, paste, and keyboard modes match their initial state. | PTY / `native-pty` |
| `TERM-002` | P0 | Trigger a handled application error while active. | Error is readable and all terminal modes restore before process exit. | PTY / `native-pty` |
| `TERM-003` | P0 | Send Ctrl-C during reading. | State checkpoint policy runs, child exits within timeout, and terminal restores. | PTY / `native-pty` |
| `TERM-004` | P0 | Trigger the controlled panic harness after setup. | Panic cleanup restores modes and diagnostic output does not corrupt the alternate screen. | PTY / `native-pty` |
| `TERM-005` | P0 | Fail each terminal setup step in turn. | Previously enabled modes roll back in reverse-safe order without masking the root error. | Integration / `pr-core` |
| `TERM-006` | P1 | Resize repeatedly, including zero or tiny transient sizes. | No panic or stale cells; usable size recovers to the same logical anchor. | PTY / `native-pty` |
| `TERM-007` | P1 | Send focus, mouse, unsupported key, and paste events in reading mode. | Unsupported input is inert; paste cannot enter modes that do not accept text. | Unit / `pr-core` |
| `TERM-008` | P1 | Run ANSI output through `vt100`. | Final cells, cursor, clears, and alternate-screen exit match direct assertions. | Integration / `pr-render` |
| `TERM-009` | P1 | Terminate while a worker is active. | Worker shutdown is bounded; no child or thread remains and terminal cleanup completes. | PTY / `native-pty` |
| `TERM-010` | P1 | Repeat launch/quit through SSH and tmux. | Controls and restoration remain correct or a documented capability fallback activates. | Manual / `release` |
| `TERM-011` | P0 | Deliver raw-mode Ctrl-C on every target and external `SIGINT` on POSIX during the foundation shell. | Supported paths dispatch the quit action, exit within timeout, and restore terminal state; other process events are explicitly excluded. | PTY / `native-pty` |
| `TERM-012` | P0 | Start from the supported shell baseline and capture native kernel terminal attributes where available. | Canonical/echo and related kernel flags return to captured values; owned ANSI modes return to primary screen, visible cursor, and disabled paste/mouse/keyboard-enhancement baseline. | PTY / `native-pty` |
| `TERM-013` | P1 | Run open, navigate, resize, search, help, error, Ctrl-C, and quit through finalized SSH/tmux rows. | Exact `ENV-*` tuple passes Unicode, color downgrade, image fallback, disconnect, detach, and restoration assertions. | Manual / `release` |

## Native Keyboard Cases

Logical action mapping remains in `NAV`; these cases prove Crossterm and native
terminals deliver the promised input without requiring release events or modern
keyboard protocols.

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `KEY-001` | P1 | Send arrows, Page Up/Down, Home/End, F1, Escape, and quit/back on each Required `ENV-*` row. | Each produces the expected action exactly once and help shows the same binding. | PTY/manual / `native-pty` |
| `KEY-002` | P1 | Send Ctrl-B/Ctrl-F with terminal flow control in documented state. | Page movement arrives reliably or Stage 1 selects and documents a conflict-free alternative. | PTY/manual / `native-pty` |
| `KEY-003` | P1 | Send `gg`, lone `g`, delayed prefix, rapid prefix, and repeats. | Final `DEC-TEST-010` prefix policy is deterministic and never loses unrelated input. | PTY / `native-pty` |
| `KEY-004` | P1 | Send press, repeat, and release events with keyboard enhancements present and absent. | Essential actions depend only on baseline press semantics; release is ignored safely. | PTY / `native-pty` |
| `KEY-005` | P1 | Enter AltGr and non-Latin text in search and notes. | Characters are inserted exactly and no reading command fires. | Manual/native / `release` |
| `KEY-006` | P1 | Exercise Escape-versus-Alt ambiguity and Ctrl-C during text entry. | Back/cancel and termination follow final mode policy with no stuck prefix or partial text corruption. | PTY/manual / `native-pty` |
| `KEY-007` | P0 | Paste normal, multiline, control-containing, and over-limit text in every mode. | Paste is accepted only in text modes, sanitized by policy, bounded by `DEC-TEST-012`, and rejected elsewhere. | PTY / `security` |

## Plain-Text Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `TXT-001` | P0 | Decode valid UTF-8 without BOM. | Exact Unicode scalar content reaches the document model. | Unit / `pr-core` |
| `TXT-002` | P1 | Decode UTF-8 with BOM. | BOM is removed once and content is otherwise unchanged. | Unit / `pr-core` |
| `TXT-003` | P1 | Decode BOM-marked UTF-16LE and UTF-16BE. | Both forms produce the expected logical text with no replacement characters. | Unit / `pr-core` |
| `TXT-004` | P0 | Open invalid UTF-8 or unmarked UTF-16. | Typed encoding error identifies the reason; bytes are not silently replaced. | Unit / `pr-core` |
| `TXT-005` | P1 | Parse LF, CRLF, and CR variants of the same text. | All produce equivalent logical newlines and positions after decoding. | Unit / `pr-core` |
| `TXT-006` | P1 | Parse paragraphs with repeated intentional blank lines. | Paragraphs and deliberate blank lines survive document conversion. | Unit / `pr-core` |
| `TXT-007` | P1 | Parse empty and whitespace-only files. | Empty-reader state is stable, navigable, and clearly reported. | Integration / `pr-core` |
| `TXT-008` | P0 | Stream a file at, below, and above the configured byte limit. | Allowed files remain bounded; above-limit input fails before unbounded allocation. | Integration / `security` |
| `TXT-009` | P0 | Parse one extremely long logical line. | Processing remains bounded, makes progress, and wraps without panic. | Property / `security` |
| `TXT-010` | P1 | Open a read-only source, navigate, annotate, and quit. | Source bytes, timestamps where controllable, and permissions remain unchanged. | Integration / `pr-core` |

## Document Model and Markdown Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `MODEL-001` | P0 | Convert each supported format to the shared model. | Sections, blocks, inline spans, and source ranges satisfy model invariants. | Integration / `pr-core` |
| `MODEL-002` | P1 | Traverse headings, paragraphs, lists, quotes, code, separators, links, and images. | Reading order is deterministic and no semantic block is duplicated or omitted. | Unit / `pr-core` |
| `MODEL-003` | P0 | Construct positions at every block and valid byte boundary. | Positions resolve to the same logical content independent of viewport width. | Property / `pr-core` |
| `MODEL-004` | P0 | Attempt invalid section, block, or non-boundary offsets. | Construction rejects invalid state without panic. | Unit / `pr-core` |
| `MD-001` | P1 | Parse `FX-MD-SEMANTIC`. | Headings, paragraphs, emphasis, strong, inline code, lists, quotes, breaks, and separators map correctly. | Integration / `pr-core` |
| `MD-002` | P1 | Parse nested ordered and unordered lists. | Nesting, order, markers, and readable indentation remain deterministic. | Unit / `pr-core` |
| `MD-003` | P1 | Parse a table at wide and narrow widths. | Wide form is readable; narrow form linearizes without dropping cell content or order. | Render / `pr-render` |
| `MD-004` | P1 | Parse links, internal anchors, and image alt text. | Targets remain inert during parsing and source offsets map to visible content. | Integration / `security` |
| `MD-005` | P1 | Parse fenced and indented code from `FX-MD-CODE`. | Original code bytes, indentation, blank lines, language tag, and copy range survive visual wrapping. | Integration / `pr-core` |
| `MD-006` | P1 | Parse inline code containing punctuation and backticks. | Delimiters are removed correctly and content remains literal. | Unit / `pr-core` |
| `MD-007` | P1 | Parse tabs, long code lines, and terminal transcripts. | Display policy is deterministic; logical copy text remains unwrapped and unchanged. | Render / `pr-render` |
| `MD-008` | P0 | Parse raw HTML with scripts, remote resources, and malformed tags. | Safe semantic text may survive; active content and fetches remain inert and bounded. | Integration / `security` |
| `MD-009` | P1 | Parse malformed but recoverable Markdown constructs. | Parser output is deterministic and never loops or panics. | Property / `pr-core` |
| `MD-010` | P1 | Compare source offsets before and after conversion. | Search, link, and annotation ranges map to original logical text. | Unit / `pr-core` |
| `MD-011` | P1 | Place text, image with alt/caption, and following text in one Markdown fixture. | Resource resolves locally at the exact model position; success and every fallback preserve surrounding order and caption. | Integration/render / `pr-render` |
| `MD-012` | P0 | Open Markdown at the numeric byte/work boundaries from `DEC-TEST-012`. | Exact boundary policy applies before excessive allocation; previous state and side-effect inventory remain clean. | Integration / `security` |

## EPUB and Archive Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `EPUB-001` | P1 | Open valid `FX-EPUB2`. | Metadata, NCX navigation, manifest, and linear spine order are correct. | Integration / `pr-core` |
| `EPUB-002` | P1 | Open valid `FX-EPUB3`. | Metadata, nav document, landmarks, and linear spine order are correct. | Integration / `pr-core` |
| `EPUB-003` | P1 | Include nonlinear resources and fallback manifest items. | Canonical linear spine and exact `DEC-TEST-015` fallback item/error sequence match fixture. | Integration / `pr-core` |
| `EPUB-004` | P1 | Use missing or untrustworthy title and author metadata. | Trustworthy metadata appears; fallback filename and path remain recognizable. | Unit / `pr-core` |
| `EPUB-005` | P1 | Parse malformed but recoverable XHTML. | Meaningful text and structure remain readable with bounded warnings. | Integration / `pr-core` |
| `EPUB-006` | P0 | Open encrypted content. | Specific unsupported-encryption error occurs before resource decoding. | Integration / `security` |
| `EPUB-007` | P1 | Open fixed-layout metadata. | Specific unsupported fixed-layout message appears; no false reflow claim. | Integration / `pr-core` |
| `EPUB-008` | P1 | Follow internal chapter, fragment, and TOC links. | Destination resolves inside the document without browser confirmation. | Integration / `pr-core` |
| `EPUB-009` | P0 | Include scripts, DTDs, external entities, remote styles, and remote media. | Nothing executes or resolves; no network or host-file access occurs. | Integration / `security` |
| `EPUB-010` | P0 | Replace or mutate the source after preflight begins. | Inspected bytes remain stable or opening aborts; unchecked replacement bytes are never parsed. | Integration / `security` |
| `EPUB-011` | P1 | Convert EPUB 2 and EPUB 3 chapters containing every supported semantic element. | Exact headings, emphasis, strong, lists, quotes, code, links, separators, breaks, tables, images, captions, order, and source IDs match fixture expectations. | Integration / `pr-core` |
| `EPUB-012` | P1 | Render the semantic EPUB fixture wide and narrow. | Tables retain every cell in order, code retains logical whitespace, and unsupported CSS layout/custom fonts do not alter reading order. | Render / `pr-render` |
| `EPUB-013` | P1 | Place text, canonical archive image, alt/caption, and following text in EPUB. | Exact member resolves without extraction; placement and text order survive protocol, cell, and caption paths. | Integration/render / `pr-render` |
| `EPUB-014` | P0 | Open text-only view of an EPUB containing many images and fonts. | Unrelated binary resources remain lazy and no decoder runs before the resource becomes visible. | Integration / `security` |
| `EPUB-015` | P0 | Monitor successful EPUB reading with isolated temp/cache roots. | Resources are read directly from the inspected archive and no reconstructed member tree or sidecar is written. | Integration / `security` |
| `EPUB-016` | P0 | Replace, truncate, append, rename, hard-link modify, and symlink-swap the source at deterministic preflight/resource hooks. | Parser uses only inspected stable bytes or aborts; instrumentation proves it never reopens an unchecked path. | Integration / `security` |
| `SEC-001` | P0 | Use absolute, parent-escaping, NUL, device, backslash-ambiguous, and duplicate-normalized member names. | Every unsafe or ambiguous member is rejected before semantic parsing. | Unit / `security` |
| `SEC-002` | P0 | Use symlink, overlapping, encrypted, and unsupported-compression entries. | Archive is rejected with the matching policy error and no extraction occurs. | Integration / `security` |
| `SEC-003` | P0 | Apply the Boundary Method to compressed EPUB size at 256 MiB. | Exact inclusivity is asserted; over-policy input is rejected before full allocation. | Unit / `security` |
| `SEC-004` | P0 | Apply the Boundary Method to 10,000 ZIP members. | Exact limit behavior, iteration count, and error memory remain bounded. | Unit / `security` |
| `SEC-005` | P0 | Apply the Boundary Method to advertised and actual total expansion at 512 MiB. | Actual bytes are counted; dishonest metadata and aggregate tiny entries cannot bypass the limit. | Integration / `security` |
| `SEC-006` | P0 | Apply the Boundary Method independently to container, OPF, NCX, and nav resources at 16 MiB. | Over-limit control files fail before XML parser allocation. | Unit / `security` |
| `SEC-007` | P0 | Apply the Boundary Method to one XHTML chapter at 32 MiB. | Final `DEC-TEST-016` policy has one exact typed result and leaves other state valid. | Integration / `security` |
| `SEC-008` | P0 | Apply the Boundary Method to the final `DEC-TEST-003` ratio and small-file rules. | Actual per-entry and aggregate counts enforce formula, rounding, zero-byte, and exception policy without overflow. | Unit / `security` |
| `SEC-009` | P0 | Independently vary XML depth at 256 and nodes at 1,000,000 for each control-document type. | Wide/shallow and narrow/deep over-limit input stops before semantic state; DTD/entity variants never resolve. | Integration / `security` |
| `SEC-010` | P0 | Deterministically feed truncated headers, central directory corruption, CRC failures, trailing data, and a fixed mutation table. | Typed malformed-archive errors occur without panic, hang, extraction, or partial trusted model. | Integration / `security` |
| `SEC-011` | P0 | Test drive-relative, UNC/verbatim, alternate separators, dot segments, trailing dots/spaces, ADS, reserved names, empty names, case/Unicode collisions, and file/directory collisions. | Each input has one host-independent canonical archive key or typed rejection before semantic parsing. | Unit / `security` |

## Image Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `IMG-001` | P1 | Decode one bounded fixture per enabled PNG/APNG, JPEG, GIF, WebP, BMP, ICO, TIFF, PNM, TGA, QOI, DDS, OpenEXR, HDR, and Farbfeld decoder. | Supported inputs normalize to expected dimensions and representative pixels. | Integration / `security` |
| `IMG-002` | P1 | Decode animated PNG and GIF. | Only the first frame is used; no animation timer or unbounded frame storage starts. | Unit / `pr-core` |
| `IMG-003` | P1 | Rasterize safe SVG and SVGZ. | Static output is bounded and scripts or animation do not execute. | Integration / `security` |
| `IMG-004` | P0 | SVG references network, absolute, parent-escaping, device, and host-file resources. | Resolver rejects each reference; no external I/O occurs. | Integration / `security` |
| `IMG-005` | P0 | Apply the Boundary Method to compressed raster input at 32 MiB and actual decompressed SVG/SVGZ XML at 8 MiB. | Byte limits are counted from actual reads before expensive decode or rasterization. | Unit / `security` |
| `IMG-006` | P0 | Apply the Boundary Method independently to width and height at 16,384 pixels. | Over-limit dimensions fail before image allocation. | Unit / `security` |
| `IMG-007` | P0 | Apply the Boundary Method to decoded area at 64 million pixels and allocation at 256 MiB. | Checked arithmetic prevents overflow and over-budget allocation. | Unit / `security` |
| `IMG-008` | P1 | Evaluate an ordered table of explicit override and simultaneous Kitty, Sixel, and iTerm2 evidence. | Exactly one backend is selected in override, Kitty, Sixel, iTerm2, cells, caption order; unselected protocol bytes are absent. | Unit / `pr-core` |
| `IMG-009` | P0 | Remove native protocol evidence while varying true-color cell support. | True-color cells select half blocks; no usable cell backend selects caption with exact alt text, dimensions, and reason. | Unit / `security` |
| `IMG-010` | P1 | Set each explicit protocol, cell, and caption override. | Valid override wins; incompatible override follows the single outcome resolved by `DEC-TEST-014`. | Unit / `pr-core` |
| `IMG-011` | P1 | Render through true-color half blocks and a 256-color terminal. | Cell bounds, aspect handling, and cleanup are correct with no stale image cells. | Render / `pr-render` |
| `IMG-012` | P1 | Force decoder, resolver, resize, and protocol failures. | Surrounding text remains readable and caption includes alt text, dimensions, and short reason. | Render / `pr-render` |
| `IMG-013` | P0 | Queue more image jobs than capacity and navigate away. | Queue remains bounded; stale generations are discarded and current work completes. | Integration / `security` |
| `IMG-014` | P1 | Switch to Paper theme around an image. | Source pixels remain unchanged; only frame, caption, placeholder, and cell background adopt theme roles. | Render / `pr-render` |
| `IMG-015` | P0 | Feed truncated, corrupt, concatenated, false-size, and high-ratio SVGZ streams. | Actual decompressed XML enforces 8 MiB, checksum errors are typed, and no XML/resource work begins after violation. | Integration / `security` |
| `IMG-016` | P0 | Deterministically feed SVG depth/node extremes and fixed pathological paths, filters, geometry, and transforms. | Parser/rasterizer respects recorded structural and work budgets, cancellation, and allocation limits. | Integration/property / `security` |
| `IMG-017` | P1 | Feed malformed, partial, spoofed, delayed, and absent terminal capability responses. | Query times out by policy and falls back once without emitting competing graphics protocols. | Integration / `native-pty` |
| `IMG-018` | P1 | On one finalized native terminal per claimed protocol, display, resize, scroll, replace, navigate away, fail, and exit. | Image appears correctly; framing/chunks/IDs are accepted; stale images are deleted and terminal remains clean. | Manual / `release` |

## Layout and Unicode Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `LAY-001` | P0 | Layout widths from minimum through 200 cells. | No rendered row exceeds available content cells. | Property / `pr-core` |
| `LAY-002` | P0 | Layout combining sequences, ZWJ emoji, flags, and skin tones. | No grapheme cluster is split or clipped midway. | Property / `pr-core` |
| `LAY-003` | P1 | Layout CJK and punctuation near line boundaries. | Unicode line-break opportunities are used deterministically. | Unit / `pr-core` |
| `LAY-004` | P1 | Layout tabs and control characters. | Defined tab expansion and safe visible control handling do not corrupt terminal output. | Unit / `pr-core` |
| `LAY-005` | P0 | Map every rendered span back to logical source ranges. | Ranges are ordered, in bounds, and reconstruct visible logical content. | Property / `pr-core` |
| `LAY-006` | P0 | Resize wide to narrow to wide. | First meaningful visible passage retains the same logical anchor. | Property / `pr-core` |
| `LAY-007` | P1 | Change theme, status visibility, and reading mode. | Layout invalidates only relevant caches and retains logical anchor. | Integration / `pr-render` |
| `LAY-008` | P0 | Layout empty blocks, one-cell widths, oversized indentation, and long unbreakable content. | Algorithm makes progress without underflow, overflow, loop, or panic. | Property / `security` |
| `LAY-009` | P1 | Linearize wide tables at narrow width. | Every cell appears in reading order with row association preserved. | Render / `pr-render` |
| `LAY-010` | P1 | Render code blocks with indentation and long lines. | Display follows selected wrap/pan policy while logical copied content remains original. | Render / `pr-render` |
| `LAY-011` | P1 | Test ambiguous-width setting values. | Cache key and measured rows change deterministically without changing source positions. | Unit / `pr-core` |
| `LAY-012` | P1 | Render below minimum supported terminal size, then recover. | Clear size message replaces unsafe layout; recovery returns to prior anchor. | Render / `pr-render` |
| `LAY-013` | P1 | Render combining marks, CJK, ambiguous-width characters, ZWJ emoji, flags, and skin tones on every Required terminal row. | Recorded font/config and observed cell/cursor placement match support claims or the limitation is documented. | Manual / `release` |
| `LAY-014` | P1 | Open Arabic, Hebrew, and mixed-direction samples. | No panic, hang, control injection, or invalid logical range; unsupported visual ordering/search/annotation behavior is clearly limited rather than claimed. | Integration/manual / `phase-gate` |
| `LAY-015` | P1 | Start under UTF-8 English, CJK, and non-UTF-8 `C` locales. | Deterministic Unicode behavior or one clear startup limitation occurs without byte corruption. | Integration/native / `phase-gate` |

## Navigation and Reading Mode Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `NAV-001` | P1 | Move next then previous visual line away from boundaries. | Original logical anchor is recovered. | Property / `pr-core` |
| `NAV-002` | P0 | Repeatedly move next page. | Every action advances unless at end; no loop or skipped terminal state occurs. | Property / `pr-core` |
| `NAV-003` | P1 | Move previous page after next page without resize. | Reader returns to the prior page anchor under the defined paging policy. | Unit / `pr-core` |
| `NAV-004` | P1 | Navigate at document and section starts and ends. | Actions clamp safely and report boundary without invalid positions. | Unit / `pr-core` |
| `NAV-005` | P1 | Jump next/previous section and through every TOC entry. | Correct section anchor and title become active. | Integration / `pr-core` |
| `NAV-006` | P1 | Switch paged to continuous and back. | First meaningful passage remains anchored and saved position stays logical. | Integration / `pr-render` |
| `NAV-007` | P1 | Exercise conventional and Vim-style bindings in reading mode. | Each promised action fires once with no unresolved collisions. | Unit / `pr-core` |
| `NAV-008` | P0 | Enter search or note-editing mode and type binding characters. | Text is entered; reading commands do not fire until the mode exits. | Unit / `pr-core` |
| `NAV-009` | P1 | Open and close help, TOC, annotation list, and confirmation views. | Return lands on the exact prior logical passage. | Integration / `pr-render` |
| `NAV-010` | P1 | Resize between each navigation action. | Position remains valid and movement continues from the visible anchor. | Property / `pr-core` |
| `NAV-011` | P1 | Open a book without a mode override at fixed viewport. | State and status are Paged; page rows equal content viewport after exact frame/status reservation. | Integration/render / `pr-render` |
| `NAV-012` | P1 | In continuous mode, move one line forward and backward. | Each action moves exactly one visual row without snapping to a page boundary. | Unit / `pr-core` |
| `NAV-013` | P1 | Parameterize paged and continuous modes over line, page, chapter, TOC, search, bookmark, highlight, and note jumps. | Every action lands on the fixture's exact logical destination in both modes. | Integration / `pr-core` |
| `NAV-014` | P1 | Invoke current section start/end and whole document start/end separately. | Four distinct actions clamp to four exact fixture anchors and use final registered bindings. | Unit / `pr-core` |

## Search Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `SEARCH-001` | P1 | Search lowercase literal forward. | Matching is case-insensitive and begins after the current logical position. | Unit / `pr-core` |
| `SEARCH-002` | P1 | Search a literal containing uppercase. | Matching is case-sensitive. | Unit / `pr-core` |
| `SEARCH-003` | P1 | Search backward from a middle and boundary position. | Results are ordered backward and retain exact logical ranges. | Unit / `pr-core` |
| `SEARCH-004` | P1 | Reach last or first result and continue. | Wrap occurs only after a visible indication and result count remains correct. | Integration / `pr-render` |
| `SEARCH-005` | P0 | Search normalized accents and combining forms. | Normalized match maps back to valid original byte and grapheme boundaries. | Property / `pr-core` |
| `SEARCH-006` | P1 | Search terms spanning wraps but not logical block boundaries. | Visual wrapping does not change matching; all visible fragments highlight correctly. | Render / `pr-render` |
| `SEARCH-007` | P1 | Submit empty, absent, boundary-length, over-limit, and control-containing queries. | Remains Blocked until `DEC-TEST-004` supplies one exact result per class; no alternative oracle may pass. | Unit / `pr-core` |
| `SEARCH-008` | P1 | Highlight all visible matches. | Highlights do not move the saved reading anchor or hide selection state. | Render / `pr-render` |
| `SEARCH-009` | P1 | Add and clear local search history. | Only expected local state changes; cleared terms are absent after reload. | Integration / `pr-core` |
| `SEARCH-010` | P0 | Start an expensive search, change query, and resize. | Stale result generation is discarded and cannot replace current highlights. | Integration / `security` |
| `SEARCH-011` | P1 | Search `.`, `*`, `[x]`, `^`, `a+b`, and backslashes. | Every metacharacter is treated literally; no regular-expression semantics occur. | Unit / `pr-core` |
| `SEARCH-012` | P1 | Put query halves in adjacent blocks whose concatenation would match. | No cross-block result exists unless the final logical-search contract explicitly joins those block types. | Unit / `pr-core` |
| `SEARCH-013` | P1 | Search forward and backward with matches at and around the current position. | Exact strict/inclusive start policy is asserted in both directions and wrap shows result index/count once. | Unit/render / `pr-render` |
| `SEARCH-014` | P1 | Add terms past history capacity, restart, clear, and inspect two books. | Final numeric capacity, ordering, persistence, clear behavior, and absence of cross-book body indexing are exact. | Integration / `pr-core` |

## Configuration and Persistence Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `CFG-001` | P1 | Load no config. | Documented built-in defaults are used. | Unit / `pr-core` |
| `CFG-002` | P1 | Load valid TOML then apply explicit CLI options. | Precedence is defaults, config, then only supplied CLI values. | Unit / `pr-core` |
| `CFG-003` | P1 | Load unknown keys, invalid types, syntax errors, and inaccessible config. | Defined warning or typed error names the setting without losing valid state. | Unit / `pr-core` |
| `CFG-004` | P1 | Resolve paths on Linux, macOS, and Windows. | Configuration, state, data, and cache use platform-native locations. | Unit/native / `phase-gate` |
| `STATE-001` | P0 | Serialize then deserialize every supported state value. | Schema version and all supported values round-trip exactly. | Property / `pr-core` |
| `STATE-002` | P0 | Save over an existing valid state file. | Temp file is same-directory, data and file are synced per policy, and replacement is atomic. | Integration / `pr-core` |
| `STATE-003` | P0 | Inject failure through file sync and before successful rename. | Previous valid state remains byte-identical and readable; case-owned temporary debris follows registered cleanup policy. | Integration / `security` |
| `STATE-004` | P0 | Load truncated JSON, invalid JSON, wrong types, and checksum-free partial content. | Corruption is recoverable and never silently replaces data with empty state. | Integration / `security` |
| `STATE-005` | P0 | Load current, supported old, and future schema versions. | Current loads, old migrates deterministically, and future version is rejected without rewrite. | Integration / `pr-core` |
| `STATE-006` | P0 | Save and restore positions across width, height, mode, and theme changes. | Same logical passage returns; visual page number is never persisted as identity. | Integration / `pr-core` |
| `STATE-007` | P1 | Source changes slightly, moves, disappears, or becomes inaccessible. | Recovery policy is deterministic and stale state remains explainable, not destructive. | Integration / `pr-core` |
| `STATE-008` | P1 | Exit normally and through supported interruption with dirty state. | Checkpoint policy saves at most expected data and reports pending or failed save. | PTY / `native-pty` |
| `STATE-009` | P0 | Run two state-changing operations or processes against one destination. | Defined locking or last-writer policy prevents malformed JSON and silent partial merge. | Integration / `security` |
| `STATE-010` | P1 | Inspect permissions of newly created state and temp files. | Platform-appropriate private defaults are used where controllable. | Integration/native / `release` |
| `STATE-011` | P0 | Fail parent-directory sync after successful atomic replacement. | Destination is a complete old or new state, never partial; typed durability error avoids claiming crash durability and no unsafe rollback is attempted. | Integration/native / `security` |
| `STATE-012` | P0 | Load state/config at byte, nesting, string, entry-count, and total-collection limits from `DEC-TEST-012`. | Over-limit input fails before excessive allocation and does not rewrite or replace prior state. | Integration / `security` |
| `STATE-013` | P0 | Present state destination as symlink, hard link, directory, device, FIFO, collision, or path swapped at deterministic hooks. | Unsafe destination types/races fail without writing outside the approved directory or following attacker-controlled replacements. | Integration/native / `security` |
| `STATE-014` | P0 | Race migration, save, clear, and delete; terminate lock owner where applicable. | Final `DEC-TEST-007` policy linearizes operations, recovers stale ownership, and never produces malformed or resurrected state. | Integration/native / `security` |
| `STATE-015` | P0 | After failed writes and logical deletion, inventory temp, backup, lock, cache, and state artifacts. | Only policy-approved artifacts remain and deleted records cannot reappear after loading any application-managed active file. | Integration / `security` |
| `STATE-016` | P1 | Open TXT, Markdown, and EPUB first with no position, then save/restart through CLI and recents with a second book present. | First open starts at exact beginning; each book restores its own exact passage; stale invalid anchor follows final recovery policy. | Integration/PTY / `phase-gate` |

## Recent Book Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `RECENT-001` | P1 | Open books in repeated and distinct order past capacity. | List is bounded, deduplicated, and most-recently-used. | Unit / `pr-core` |
| `RECENT-002` | P1 | Reopen a valid recent entry. | Book opens at its last valid logical position. | Integration / `pr-core` |
| `RECENT-003` | P1 | Show trustworthy and missing metadata. | Title/author or recognizable filename/path fallback displays correctly. | Render / `pr-render` |
| `RECENT-004` | P1 | Entry is moved, missing, unreadable, or unsupported. | Entry is marked without repeated automatic failures or removal. | Integration / `pr-core` |
| `RECENT-005` | P0 | Remove one entry. | Only recent state changes; source file and annotations remain untouched. | Integration / `pr-core` |
| `RECENT-006` | P0 | Clear list, cancel once, then confirm. | Cancel preserves list; confirmation clears recents without deleting any source or annotation. | Integration / `pr-render` |
| `RECENT-007` | P0 | Start with a directory containing many books. | No automatic crawl, watcher, metadata download, or hidden index occurs. | Integration / `security` |
| `RECENT-008` | P1 | Open another local path from the screen. | Path handling matches CLI safety and successful open updates MRU once. | PTY / `native-pty` |

## Bookmark and Annotation Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `ANN-001` | P1 | Create, name, rename, list, jump to, and delete a bookmark. | Each operation updates only local state and jump resolves exact logical location. | Integration / `pr-core` |
| `ANN-002` | P1 | Create a highlight over ASCII, combining, emoji, and cross-wrap text. | Range uses valid logical boundaries and remains correct after relayout. | Property / `pr-core` |
| `ANN-003` | P1 | Select each allowed accessible highlight color. | Stored value is valid and visual state includes a non-color distinction where needed. | Render / `pr-render` |
| `ANN-004` | P1 | Create and edit point and range notes with multiline plain text. | Text round-trips exactly as plain data and never executes as markup. | Integration / `pr-core` |
| `ANN-005` | P1 | List mixed bookmarks, highlights, and notes. | Passage preview, chapter context, type, and stable order are correct. | Render / `pr-render` |
| `ANN-006` | P1 | Jump to an item and invoke return. | Jump lands on exact item range and final `DEC-TEST-017` stack returns to one exact prior anchor. | Integration / `pr-core` |
| `ANN-007` | P0 | Cancel then confirm deletion. | Cancel changes nothing; confirmation removes only selected item atomically. | Integration / `pr-core` |
| `ANN-008` | P1 | Reopen after source insertion, deletion, move, or identity change. | Recoverable ranges relocate by defined evidence; unresolved ranges remain visible and safe. | Integration / `pr-core` |
| `ANN-009` | P0 | Use note text containing escapes, control text, URLs, and markup. | Content remains inert plain text and terminal-safe rendering escapes controls. | Integration / `security` |
| `ANN-010` | P0 | Compare source hash before and after every annotation operation. | TXT, Markdown, and EPUB source bytes never change. | Integration / `security` |
| `ANN-011` | P1 | Create mixed annotations in two books and open each management view. | Only the current book's items appear, with exact names, colors, text, previews, context, and order. | Integration/render / `pr-render` |
| `ANN-012` | P1 | Create named bookmark, colored highlight, point note, and range note; restart; change highlight color; restart again. | Every field and logical range round-trips, and the changed color plus non-color cue renders after reload. | Integration/render / `pr-render` |
| `ANN-013` | P1 | Exercise bookmark create, rename, jump, and delete as separate steps. | Exact intermediate list and stored state are asserted after each operation. | Integration / `pr-core` |
| `ANN-014` | P1 | Apply fixed insert/delete/move edits to bookmarked, highlighted, point-note, and range-note passages. | Final `DEC-TEST-006` policy either relocates to the exact expected text or retains an unresolved item with reason and never attaches unrelated text. | Integration / `pr-core` |

## Theme and Rendering Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `THEME-001` | P1 | Render dark, light, high-contrast, monochrome, and Paper themes. | Semantic roles map consistently and content remains readable. | Render / `pr-render` |
| `THEME-002` | P1 | Load each theme name from TOML, switch theme in session, and restart without rewriting config. | Configured theme loads; session selection follows explicit persistence policy and never silently edits user TOML. | Integration / `pr-core` |
| `THEME-003` | P0 | Switch themes at the middle of a wrapped passage. | Logical anchor, selection, search range, and annotation range remain unchanged. | Integration / `pr-render` |
| `THEME-004` | P1 | Render Paper in true color. | Exact semantic palette roles and at least 4.5:1 text contrast are asserted. | Render / `pr-render` |
| `THEME-005` | P1 | Render Paper in 256-color, terminal-default, and `NO_COLOR`. | Nearest/fallback roles preserve contrast and every state remains distinguishable. | Render / `pr-render` |
| `THEME-006` | P1 | Reduce Paper from wide through minimum viewport. | Outer canvas shrinks first, page padding second, boundary last; content minimum is preserved. | Render / `pr-render` |
| `THEME-007` | P1 | Render focus, selection, search, links, warnings, and annotation colors. | Color is paired with underline, bold, reverse, label, or another non-color cue. | Render / `pr-render` |
| `THEME-008` | P1 | Render images and failure captions in Paper. | Original image pixels remain unchanged and surrounding presentation uses Paper roles. | Render / `pr-render` |
| `THEME-009` | P1 | Run pairwise Paper matrix over true color, 256, 16/default, `NO_COLOR`; wide, ordinary, narrow, minimum, below minimum; and every reader/UI state. | Text, bounds, non-color cues, exact link underline, anchor, and collapse order pass direct assertions before snapshot approval. | Render / `pr-render` |
| `THEME-010` | P1 | Compute every application-controlled Paper foreground/background pairing. | Exact palette values are used and every text pairing is at least 4.5:1; terminal-default palettes make no universal contrast claim. | Unit / `pr-core` |
| `RENDER-001` | P1 | Render reader at `120x40`, `80x24`, `40x10`, and minimum size. | Direct cell bounds pass and reviewed snapshots match intentional output. | Render / `pr-render` |
| `RENDER-002` | P1 | Render empty, short, long, malformed, and loading/error states. | No stale cells, clipping, hidden errors, or cursor artifacts occur. | Render / `pr-render` |
| `RENDER-003` | P1 | Render ASCII, combining marks, CJK, emoji, tabs, and safe control representations. | Cell widths and source mappings match direct assertions. | Render / `pr-render` |
| `RENDER-004` | P1 | Redraw unchanged state and one-field changes. | Output is stable; dirty policy avoids unnecessary redraw without missing updates. | Unit/render / `pr-render` |

## Status, Help, and Error Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `STATUS-001` | P1 | Render full status at wide width. | Title, chapter, logical location, dynamic page, percent, clock, mode, and message are correct. | Render / `pr-render` |
| `STATUS-002` | P1 | Reduce width through each collapse point. | Final `DEC-TEST-011` order removes exact fields without truncating essential messages unsafely. | Render / `pr-render` |
| `STATUS-003` | P1 | Advance, resize, and change mode. | Percentage is monotonic for forward movement; dynamic page updates but is never persisted. | Unit / `pr-core` |
| `STATUS-004` | P1 | Trigger confirmation, warning, search count, and pending-save messages. | Final `DEC-TEST-011` tick/input lifetime and replacement fields pass; prior status returns afterward. | Unit/render / `pr-render` |
| `STATUS-005` | P1 | Use fixed fixture, viewport, fake clock, and known anchor. | Exact title, chapter, location, page, percentage, time, mode, and message strings match expected values. | Unit/render / `pr-render` |
| `STATUS-006` | P1 | Step through each width using final `DEC-TEST-011` collapse order. | Exact fields disappear in order; temporary messages replace named lower priorities and restore after deterministic ticks/input policy. | Render / `pr-render` |
| `STATUS-007` | P1 | Resize without moving and trigger failed save. | Layout-derived page changes while logical location/percentage remain; failed-save state is visible for final deterministic lifetime. | Render / `pr-render` |
| `HELP-001` | P1 | Open help from every interaction mode. | Current-mode keys, commands, fallbacks, and status explanations are present and accurate. | Render / `pr-render` |
| `HELP-002` | P1 | Navigate or search help, then close it. | Help is keyboard-complete and returns to exact prior logical passage and mode. | Integration / `pr-render` |
| `HELP-003` | P1 | Compare registered essential actions with help entries. | Every essential action and active default binding appears exactly once in appropriate context. | Unit / `pr-core` |
| `HELP-004` | P1 | Enumerate recent, reading, search-entry, note-editing, TOC, annotations, confirmation, and help contexts. | Each context shows only valid actions plus image fallback order, color/accessibility guidance, status definitions, and known limitations. | Unit/render / `pr-render` |
| `HELP-005` | P1 | Exercise the final `DEC-TEST-009` help navigation model. | Search queries/results/no-result/exit or section reachability has one exact oracle and returns to the prior context. | Integration / `pr-render` |
| `ERR-001` | P1 | Trigger each typed domain error. | Message states what failed, safe context, reason, and possible next action. | Unit / `pr-core` |
| `ERR-002` | P0 | Trigger error while alternate screen is active. | Raw debug chain never overwrites active UI; cleanup precedes terminal-safe diagnostic. | PTY / `native-pty` |
| `ERR-003` | P0 | Include terminal control bytes and private note text in failing input. | Diagnostic escapes controls and excludes unrelated private content. | Unit / `security` |

## UI Interaction Cases

These cases connect the state and focus model in `ui_mockups.md` to feature
logic. Render snapshots supplement the direct state, range, and side-effect
assertions.

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `UI-001` | P1 | Enter Open Path, type/paste, trigger validation error, resize, and cancel. | `OpenPath` exclusively owns text focus; buffer/cursor/error survive resize; cancel returns to prior recent row and no directory scan occurs. | Integration/render / `pr-render` |
| `UI-002` | P1 | Enter link focus with internal, external, and unsupported visible links; move, resize, activate, and cancel. | Focus follows logical order with non-color cue; resize retains logical link; each class takes its exact navigation/confirmation/inert path. | Integration/render / `pr-render` |
| `UI-003` | P1 | Start, extend, reverse, resize, and cancel a grapheme-rich logical selection. | Range endpoints remain valid logical boundaries; non-color selection survives relayout; cancel creates no state and restores anchor. | Property/render / `pr-render` |
| `UI-004` | P1 | Create highlight, range note, and point note from reader interaction. | Each action opens the correct dialog/editor with exact attachment type and source range; text-entry mode suppresses reader commands. | Integration / `pr-render` |
| `UI-005` | P1 | Open search history, choose/remove a term, cancel clear, confirm clear, and return. | Bounded local order, current query, focus, persistence, and destructive confirmation behavior match `SEARCH-014`; no cross-book index appears. | Integration/render / `pr-render` |
| `UI-006` | P1 | Open theme view, preview every theme, cancel once, apply for session, show config path, and restart. | Preview/apply preserve all logical state; cancel restores prior theme; TOML is never silently rewritten; configured startup theme follows `THEME-002`. | Integration/render / `pr-render` |
| `UI-007` | P0 | Inspect boundary-length path, URL, and diagnostic at standard, narrow, and minimum usable size. | Entire escaped value is reachable through vertical movement, exact validated value is retained, and no shortened string is launched or persisted as identity. | Render/integration / `security` |
| `UI-008` | P0 | Cross below minimum size from every focus/input/selection/confirmation mode, including zero and one cell, then recover. | No invalid/clipped output or accidental command occurs; buffer, cursor, target, focus, range, and origin restore exactly. | Property/PTY / `native-pty` |
| `UI-009` | P0 | Cancel and confirm remove recent, clear recents/history, and annotation deletion under key repeat and save failure. | Cancel has initial focus; operation runs at most once; wording names retained data; failed atomic save never reports success. | Integration/render / `pr-render` |
| `UI-010` | P0 | Transition one visible image placeholder to native, cell, caption, and failure outcomes with height changes. | First meaningful logical anchor is compensated, surrounding order remains, and obsolete protocol/cell output is cleared. | Integration/render / `pr-render` |
| `UI-011` | P1 | Generate help for every mode and status indicator from action/status registries. | Required context actions and glossary entries appear once, provisional navigation aside; registered input and help bindings cannot drift. | Unit/render / `pr-render` |
| `UI-012` | P0 | Trigger pre-UI setup failure, command-path open failure, in-app path validation, unsupported book, and fatal active-session error. | Each transition chooses its exact plain or in-app recovery surface and terminal restoration occurs before plain diagnostics. | Integration/PTY / `native-pty` |

## Link, Privacy, and Data Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `LINK-001` | P0 | Parse HTTP, HTTPS, mail, file, JavaScript, data, malformed, and unknown schemes. | Only explicitly supported external schemes become openable; all remain inert during parsing. | Unit / `security` |
| `LINK-002` | P0 | Activate a supported external link. | Full destination is visible and browser launch requires explicit confirmation. | Integration / `pr-render` |
| `LINK-003` | P0 | Cancel external-link confirmation. | No process or network activity occurs and reader returns to exact passage. | Integration / `security` |
| `LINK-004` | P1 | Confirm on Linux, macOS, and Windows. | Correct system launcher receives exactly one validated URL with no shell interpolation. | Integration/native / `release` |
| `LINK-005` | P0 | Activate internal EPUB target. | Navigation stays inside document and never invokes browser confirmation. | Integration / `pr-core` |
| `LINK-006` | P0 | Classify mixed-case/obfuscated schemes, whitespace, CR/LF, NUL, tabs, escapes, encoded delimiters, user-info, IDN, ports, fragments, leading dash, and boundary-length URLs. | Final `DEC-TEST-008` table yields one typed decision; display is terminal-safe and parsing launches nothing. | Unit / `security` |
| `LINK-007` | P0 | Display a validated URL, mutate source state, then confirm with rapid repeated activation. | Exact displayed validated bytes are passed once as one fake-launcher argument; no shell, option injection, or duplicate launch occurs. | Integration / `security` |
| `LINK-008` | P0 | Cancel confirmation for long and suspicious but displayable URLs. | No launch/network occurs, exact escaped destination remains visible/selectable by policy, and reader anchor is unchanged. | Integration/render / `pr-render` |
| `LINK-009` | P1 | Inject launcher missing, spawn error, timeout, and child failure. | One actionable error appears, no retry loop/network assumption occurs, and terminal/reader state remains valid. | Integration/native / `release` |
| `PRIV-001` | P0 | Monitor sockets while opening and reading every supported local format. | TermLeaf initiates no network connection or DNS resolution. | Integration / `security` |
| `PRIV-002` | P0 | Monitor filesystem access from startup without path and with one path. | Only executable resources, explicit book, config/state paths, and required platform files are accessed; no library scan occurs. | Integration / `security` |
| `PRIV-003` | P0 | Compare all source files before and after full reader journeys. | Source bytes are unchanged. | Integration / `security` |
| `PRIV-004` | P0 | Inspect persisted state after reading, searching, and annotating. | Only documented settings, positions, recents, search history, and annotations exist; no telemetry identifier appears. | Integration / `security` |
| `PRIV-005` | P1 | Clear recents and search history and delete annotations. | Removed data is absent from active state and subsequent serialization; source remains untouched. | Integration / `pr-core` |
| `PRIV-006` | P0 | Trigger errors involving several unrelated books and notes. | Error and optional diagnostics include only context required for the failing operation. | Integration / `security` |
| `PRIV-007` | P0 | Redirect every platform directory, seed private canaries, and run successful/failing journeys. | Created-file and process inventory contains no default log, crash report, upload, hidden catalog, unadvertised cache, or private canary outside documented fields. | Integration / `security` |
| `PRIV-008` | P0 | Seed sibling/home/removable paths, denied trees, loops, and many unrelated books while monitoring access. | Exact per-platform allowlist records no crawl, watcher, metadata service, resolver, proxy, or unrelated file access. | Integration/native / `security` |
| `PRIV-009` | P0 | Record identity, size, bytes, mode, stable timestamps, directory entries, and supported xattrs/ACLs around all format and failure journeys. | Sources, referenced resources, siblings, permissions, metadata, and sidecars remain unchanged. | Integration/native / `security` |
| `PRIV-010` | P0 | Validate current state against versioned allowed-field schema. | No telemetry ID, hostname, username, machine ID, excerpt, note/search content outside documented fields, or undocumented fingerprint is persisted. | Integration / `security` |

## Concurrency and Cancellation Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `CON-001` | P0 | Submit work beyond each queue capacity. | Final `DEC-TEST-018` backpressure/rejection result and numeric capacity are exact; memory remains within registered bound. | Integration / `security` |
| `CON-002` | P0 | Complete old and new generations out of order. | Only current generation mutates application state. | Unit / `pr-core` |
| `CON-003` | P1 | Navigate away while image, parse, layout, or search work is active. | Stale work is discarded promptly and visible state remains current. | Integration / `pr-core` |
| `CON-004` | P0 | Inject worker panic, disconnect, and decode error. | Failure becomes typed error or fallback; UI never waits forever. | Integration / `security` |
| `CON-005` | P0 | Shut down with full queues and blocked work. | All threads and child work terminate within timeout without skipping terminal cleanup. | Integration/PTY / `native-pty` |
| `CON-006` | P1 | Run coordination tests under high repetition. | No arbitrary sleeps, data races, deadlocks, or order-dependent failures occur. | Scheduled / `scheduled` |
| `CON-007` | P0 | Instrument locks during I/O, decode, rendering, and channel sends. | No shared lock remains held across potentially blocking operations. | Unit/review / `security` |
| `CON-008` | P0 | Saturate request and result channels with repeated cancel/requeue cycles. | Registered numeric capacities, worker/thread maximum, in-flight bytes, and accepted/rejected/completed accounting never exceed policy. | Integration / `security` |
| `CON-009` | P0 | Cancel expensive stale parse/search/image work and overflow generation counter in harness. | Cancellation reaches registered checkpoints within deadline; generation rollover cannot make stale work current. | Integration / `security` |

## Accessibility Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `A11Y-001` | P0 | Complete every essential journey using keyboard only. | Open, read, navigate, search, annotate, help, confirm, cancel, and exit need no mouse. | PTY/manual / `release` |
| `A11Y-002` | P1 | Render high-contrast and monochrome modes. | Essential text meets defined contrast and every state remains distinguishable without hue. | Render / `pr-render` |
| `A11Y-003` | P1 | Set `NO_COLOR`. | Output uses terminal defaults and text attributes without raw decorative color sequences. | Integration / `pr-render` |
| `A11Y-004` | P1 | Inspect errors, temporary messages, and focus movement. | Messages persist long enough by deterministic policy and keyboard focus is visible. | Render/manual / `phase-gate` |
| `A11Y-005` | P1 | Run available plain-text or noninteractive output paths. | Useful content and errors remain available outside the full-screen visual UI. | Integration / `phase-gate` |
| `A11Y-006` | P1 | Exercise supported screen readers on each claimed platform. | Documented journeys are understandable; limitations are recorded without unsupported claims. | Manual / `release` |
| `A11Y-007` | P1 | Use non-Latin layout and AltGr while editing search or notes. | Text entry is not consumed as commands and essential controls retain accessible alternatives. | Manual/native / `release` |
| `A11Y-008` | P1 | Observe redraw behavior during ordinary reading. | No unnecessary animation or flashing occurs; content changes only in response to state. | Manual/render / `phase-gate` |
| `A11Y-009` | P1 | Run scripted open/read, navigation, search, error, help, confirmation, and exit journeys with VoiceOver/macOS, Narrator or NVDA/Windows, and Orca/Linux on finalized terminal rows. | Tester records comprehensibility checkpoints and exact limitations; absence of a tested combination cannot support a claim. | Manual / `release` |
| `A11Y-010` | P1 | Use representative terminal-default light/dark palettes and `NO_COLOR`. | Automated tests assert only non-color distinctions and absence of decorative forced colors; manual evidence records palette-specific readability without universal contrast claims. | Render/manual / `release` |

## Performance Cases

Record hardware, OS, terminal, book hash, viewport, profile, sample count, and
peak-memory method. Budgets are provisional until representative hardware is
recorded.

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `PERF-001` | P1 | Warm launch to empty reader from process invocation to first complete frame. | Median and high percentile meet 150 ms provisional budget; terminal cleanup is proved separately by `TERM-001`. | Benchmark / `phase-gate` |
| `PERF-002` | P1 | Open first page of typical TXT, Markdown, EPUB 2, and EPUB 3. | Each meets 500 ms provisional budget or documented exception. | Benchmark / `phase-gate` |
| `PERF-003` | P1 | Navigate line/page after layout. | Interaction completes within 50 ms provisional budget. | Benchmark / `phase-gate` |
| `PERF-004` | P1 | Resize typical book and preserve anchor. | Relayout completes within 100 ms provisional budget. | Benchmark / `phase-gate` |
| `PERF-005` | P0 | Save registered typical dirty state atomically on each native filesystem row. | Median and high percentile meet 50 ms provisional budget without weakening durability. | Benchmark/native / `phase-gate` |
| `PERF-006` | P1 | Read typical book through representative journey. | Peak process memory remains within 150 MiB provisional budget. | Benchmark/native / `phase-gate` |
| `PERF-007` | P0 | Open hostile near-limit files and cancel stale work. | Registered peak memory, completion, operation-budget, and cancellation ceilings pass; correctness timeout remains separate from benchmark comparison. | Benchmark / `security` |
| `PERF-008` | P1 | Search registered tiny, typical, large, CJK, emoji, malformed, and limited RTL fixtures. | Median/high percentile and scaling slope stay within recorded regression threshold; otherwise case is Informational, not Passing. | Benchmark / `scheduled` |
| `PERF-009` | P1 | Resize and switch chapters for registered iteration and steady-state windows. | Cache reaches bounded steady state and retained memory growth stays below registered threshold. | Benchmark / `scheduled` |

## Benchmark Method

Before a performance case can become Passing, its registry entry fixes:

- Hardware model, CPU governor/power state, memory, storage, OS, filesystem,
  terminal, viewport, and release profile.
- Fixture ID, SHA-256, compressed/logical size, section count, and cache state.
- Start and end markers such as process invocation, terminal ready, source
  accepted, first complete frame, action dispatch, and durable-save completion.
- Warm/cold policy, warmup count, sample count, median, high percentile,
  confidence or noise rule, peak-memory method, and regression threshold.
- Correctness timeout and cancellation deadline, which do not move merely
  because benchmark hardware is slower.

Shared CI timing is informational. A budget gate uses recorded representative
hardware and must link paired correctness cases for terminal cleanup, durability,
anchor preservation, and bounded allocation.

## Supply Chain and Release Cases

| ID | Priority | Setup and action | Pass criteria | Layer/profile |
| --- | --- | --- | --- | --- |
| `SUP-001` | P0 | Resolve with committed lockfile and `--locked`. | Build never changes lockfile or resolves an undeclared source. | CI / `pr-core` |
| `SUP-002` | P0 | Run advisories, licenses, sources, and bans policy. | `cargo deny check` passes or exception records reason, scope, risk, compensating tests, owner, expiry, and removal condition. | CI / `security` |
| `SUP-003` | P1 | Inspect default and all-feature dependency trees. | No unexpected runtime, network, native, duplicate-major, or license obligation appears. | Review / `security` |
| `SUP-004` | P0 | Inspect GitHub Actions definitions. | Actions are pinned to reviewed commits and permissions are read-only unless justified per job. | Review / `release` |
| `SUP-005` | P1 | Generate third-party notices. | Cargo and non-Cargo assets have complete compatible provenance and notices. | CI/review / `release` |
| `SUP-006` | P0 | Inspect `Cargo.toml`, LICENSE, notices, and package metadata. | Final GPL SPDX choice and copyright/license references are exact and consistent before Rust initialization. | Review / `phase-gate-0` |
| `SUP-007` | P1 | Review each dependency addition/upgrade and lockfile diff. | Need, maintenance, unsafe/build scripts/proc macros, default features, advisories, license, source, native/platform impact, duplicate majors, and test-only placement are recorded. | Review / `security` |
| `SUP-008` | P0 | Inspect enabled features and deferred crate categories. | `ratatui-image` defaults/Chafa and unapproved network, async, database, watcher, config-framework, native, or decoder features are absent unless an explicit decision changed scope. | Review / `security` |
| `SUP-009` | P1 | Validate enabled image-decoder registry. | Every decoder names feature, platforms, dependencies, unsafe/native status, license, limits, fixtures, fuzz corpus, and approval. | Review / `security` |
| `SUP-010` | P0 | Inspect release workflow trigger and identity. | Artifacts can originate only from protected tags/commits and privileged jobs have minimum permissions. | Review / `release` |
| `REL-001` | P0 | Build release artifacts natively on each promised OS. | Locked source builds, starts, reads a fixture, and exits cleanly. | Native CI / `release` |
| `REL-002` | P0 | Install on clean supported OS accounts. | Published instructions install without undeclared tools and first reading journey passes. | Manual/native / `release` |
| `REL-003` | P1 | Upgrade from previous supported release with existing config and state. | Migration preserves supported data and rollback procedure remains available. | Manual/native / `release` |
| `REL-004` | P0 | Verify archive and installer checksums. | Published checksums match artifacts and tampered files fail verification. | CI / `release` |
| `REL-005` | P1 | Trace artifact to tag, revision, lockfile, source, and notices. | Every artifact has complete provenance and version output matches tag. | Review / `release` |
| `REL-006` | P0 | Run native terminal matrix smoke journeys. | Every claimed platform opens, navigates, resizes, restores terminal, and exits. | Manual/native / `release` |
| `REL-007` | P1 | Review known limitations. | Unicode, bidi, image, terminal, and accessibility claims match passed evidence. | Review / `release` |
| `REL-008` | P0 | For every Required `ENV-*` platform row, run native locked build, core/doctests, PTY lifecycle, installation, and first-reading journey. | Evidence manifest binds OS, architecture, terminal, artifact hash, revision, and exact passed IDs; a missing tuple blocks that platform claim. | Native CI / `release` |
| `REL-009` | P1 | Enumerate expected archive, installer, checksum, manifest, source/build instructions, and notice files per platform. | Complete artifact set exists and embedded version/source references match protected tag. | Integration/review / `release` |
| `REL-010` | P1 | Build the same protected source twice in controlled native environments. | Archive/binary differences are absent or retained and explained sufficiently to investigate reproducibility. | Integration / `release` |
| `REL-011` | P1 | Upgrade from a named supported predecessor and execute rollback. | State/config migrate, supported data remains, rollback procedure works; explicitly Not applicable for first release without predecessor. | Manual/native / `release` |
| `REL-012` | P1 | Run `cargo-about` and reconcile Cargo plus asset provenance. | Distributed notice matches resolved graph and every shipped non-Cargo asset. | Integration/review / `release` |

## Property Test Catalog

| ID | Generator | Invariant |
| --- | --- | --- |
| `PROP-001` | Valid logical documents and widths | No row exceeds content width and layout always terminates. |
| `PROP-002` | Unicode strings with grapheme-rich content | Layout never splits a grapheme cluster. |
| `PROP-003` | Documents, anchors, and resize sequences | Logical anchor remains valid and points to the same passage. |
| `PROP-004` | Documents and navigation action sequences | Positions stay in bounds; next page progresses unless at end. |
| `PROP-005` | Search text, normalization variants, and queries | Every match maps to ordered valid original ranges. |
| `PROP-006` | Supported state values | Serialize then deserialize preserves values and schema. |
| `PROP-007` | Archive metadata counts and sizes near limits | Checked totals never overflow and policy decisions are monotonic. |
| `PROP-008` | Image dimensions, channels, and byte sizes | Allocation calculations never overflow or exceed accepted limits. |
| `PROP-009` | Event and worker completion orderings | Stale generations never replace current state. |
| `PROP-010` | Valid application actions from valid states | State transition returns valid state or typed error without panic. |

Store minimized regressions as focused fixtures when they expose a distinct bug.
CI uses deterministic case counts; scheduled runs increase counts and retain the
seed for every failure.

## Fuzz Target Catalog

These IDs describe optional coverage-guided discovery targets in the `weekly`
profile. They are not security requirements or phase-gate members. Each target
must link through `implements` to deterministic behavioral cases that remain
required whether or not fuzz discovery is configured or run.

| ID | Target | Required assertions |
| --- | --- | --- |
| `FUZZ-001` | Plain-text detection and decoding | No panic, excessive allocation, or unsupported silent replacement |
| `FUZZ-002` | ZIP preflight | No panic, hang, extraction, path escape, or limit bypass |
| `FUZZ-003` | EPUB container, OPF, NCX, and nav parsing | Bounded typed result with no external resolution |
| `FUZZ-004` | XHTML and raw HTML conversion | No active content, panic, nontermination, or invalid source range |
| `FUZZ-005` | Markdown event-to-model conversion | Valid model or bounded error with valid offsets |
| `FUZZ-006` | SVG/SVGZ resolver and rasterization boundary | No network, host read, over-budget allocation, or panic |
| `FUZZ-007` | Enabled raster decoder boundary | Limits apply before normalized allocation and failures remain bounded |
| `FUZZ-008` | State JSON loading and migration | No panic or silent rewrite; unsupported versions remain intact |
| `FUZZ-009` | Configuration TOML loading | No panic; invalid values return bounded diagnostic |
| `FUZZ-010` | Action/state transition sequences | State invariants and logical positions remain valid |
| `FUZZ-011` | URL classification, display escaping, and launcher argument construction | No process/network side effect, control injection, option injection, panic, or display/argument mismatch |
| `FUZZ-012` | Archive-member canonicalization independent of ZIP parsing | No host-dependent escape, ambiguous collision acceptance, panic, or unchecked path output |

When selected, each fuzz target needs an explicit maximum input size, timeout
policy, seed corpus, dictionary where useful, and crash-artifact procedure. No
duration or resource budget is inherited from the registry. A crash becomes a
stable deterministic regression case before the defect closes.

The default harness sandbox denies network, process spawning, and writes outside
its temporary root. A selected run records its per-input timeout, RSS ceiling,
and named corpus, and may exercise enabled image decoders individually or in the
all-feature configuration. Larger limits require an explicit target reason
rather than inheriting arbitrary libFuzzer defaults.

## Phase Gates

| Phase | Required evidence before Complete |
| --- | --- |
| 0. Rust foundation | Exact foundation-owned `QG`, `CLI`, `TERM`, `SUP`, registry/profile/harness, and base-shell cases; later feature-dependent CLI/TERM cases remain assigned forward |
| 1. Plain-text reading loop | Active `TXT`, `MODEL`, `LAY`, `NAV`, core `KEY`, all `THEME`, `RENDER`, `STATUS`, `ERR`, responsive `UI`, and registered provisional performance cases |
| 2. Structured books and images | `MD`, `EPUB`, `SEC`, `IMG`, relevant `CON`/`UI`, TOC/link-focus, deterministic malformed/boundary/mutation cases, and licensed corpus journeys |
| 3. Dependable reading | `CFG`, `STATE`, `RECENT`, `SEARCH`, `ANN`, selection/editor/history `UI`, complete `HELP`, required `KEY`/`A11Y`, and named native rows |
| 4. Product refinement | `LINK`, relocation/return recovery, full Paper matrix, `PRIV`, usability, manual accessibility, and registered performance budgets |
| 5. Release | All deterministic P0/P1 behavioral cases, accepted P2 scope, `SUP`, `REL`, clean install, upgrade, notices, checksums, and known limitations; optional `FUZZ-*` discovery is not required |

A phase cannot be Complete with a failing required case. A blocked P1 case needs
an explicit scope or support decision; a blocked P0 case prevents completion.

`tests/phase_gates.toml` freezes each `phase-gate-N` membership as exact IDs and
Required `ENV-*` rows. Every gate includes all earlier gate membership and
permanent regressions. Passing evidence separately names revision, date,
blocked cases, and CI/manual artifacts; frozen membership does not imply a gate
passed. `FUZZ-*` IDs and default fuzz durations are prohibited from frozen gate
manifests; the deterministic cases linked by each fuzz target remain required.
“Applicable,” “base,” “depth,” or “accepted scope” may not appear in a frozen
manifest.

## Defect and Regression Process

1. Assign or identify the affected stable case IDs.
2. Add the smallest test that fails for the reported behavior.
3. Record the original input, environment, and failure without private data.
4. Implement the fix without weakening unrelated assertions or limits.
5. Run mapped families plus the profile required by the defect's risk.
6. Preserve a small licensed or generated fixture when the existing catalog did
   not reproduce the defect.
7. Record exact results and any skipped environment in `testreport.md`.

The regression record also includes defect ID, owner, pre-fix revision and
failing evidence, post-fix passing evidence, permanent profile assignment,
fixture/privacy review, affected releases, backport decision, and retirement
condition. Silent quarantine is prohibited. Temporary disablement is Blocked
with approver and expiry; the stable ID and original failure remain visible.

If a snapshot changes, review semantic and direct assertions before accepting
the new snapshot. If the expected behavior changes, update the product contract
and catalog instead of silently editing only the test.

## Per-Commit Selection

Every implementation commit must include a test selection statement in
`testreport.md`:

```text
Changed areas: layout, navigation
Changed paths: src/layout/viewport.rs, src/reader/navigation.rs
Selected case IDs: LAY-001..LAY-012, NAV-001..NAV-014, SEARCH-005,
  SEARCH-006, RENDER-001, RENDER-003, A11Y-002
Profiles run: pr-core, pr-render
Commands: exact manifest commands and feature flags
Required environments: hermetic host render environment
Cases skipped: A11Y-009 (release-only manual screen-reader matrix)
Result: all selected automated cases passed
Cleanup: cargo clean passed
```

Selection rules:

- Run the mapped families for every changed area.
- Add `security` for any new or changed untrusted boundary, limit, path, URL,
  persistence operation, dependency, worker, or allocation calculation.
- Add `pr-render` when cells, widths, colors, focus, status, or help can change.
- Add `native-pty` when startup, shutdown, events, signals, terminal modes, or
  process behavior can change.
- Run the regression case for every fixed defect.
- State why an apparently mapped family is not applicable. Silence is not a
  skip reason.

## Completion Rules

A catalog case is complete only when:

- Its requirement and oracle are still consistent with `project_plan.md`.
- The test fails when the protected behavior is deliberately broken or the
  boundary is crossed.
- It passes at every required layer, profile, fixture, and native environment.
- It is deterministic, isolated, bounded, and leaves no user or terminal state.
- Failure output identifies the stable case and enough safe context to debug it.
- Its fixture has provenance and licensing or deterministic generation details.
- Its latest relevant execution is recorded honestly in `testreport.md`.

The catalog itself must be reviewed whenever a feature, security limit,
platform promise, dependency boundary, or release gate changes. Adding code
without adding or selecting the corresponding case IDs does not satisfy the
TermLeaf quality standard.
