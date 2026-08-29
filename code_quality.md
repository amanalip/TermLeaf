# Rust Code Quality Standards

**Last updated:** August 20, 2026 at 12:05 AM EDT

## Table of Contents

- [Purpose](#purpose)
- [Requirement Language](#requirement-language)
- [Engineering Priorities](#engineering-priorities)
- [Rust Baseline](#rust-baseline)
- [Architecture](#architecture)
- [Types and APIs](#types-and-apis)
- [Errors and Panics](#errors-and-panics)
- [Untrusted Input](#untrusted-input)
- [Terminal Safety](#terminal-safety)
- [Concurrency](#concurrency)
- [Persistence and Privacy](#persistence-and-privacy)
- [Performance](#performance)
- [Dependencies](#dependencies)
- [Style and Documentation](#style-and-documentation)
- [Testing](#testing)
- [Required Validation](#required-validation)
- [Review Checklist](#review-checklist)
- [Exceptions](#exceptions)

## Purpose

These standards define what well-written Rust means for TermLeaf. They apply to
production code, tests, benchmarks, fuzz targets, build scripts, and examples.
They complement the architecture and security limits in `project_plan.md`.

Quality is not measured by clever syntax, the number of abstractions, or the
absence of every allocation. TermLeaf code should make reader-visible behavior,
failure handling, ownership, and resource limits easy to understand and hard to
misuse.

## Requirement Language

- **Must** is required for a change to be accepted.
- **Should** is the default; a different choice needs a concrete reason.
- **May** identifies an available option, not required work.

When two correct designs satisfy these standards, prefer the smaller design
with fewer types, dependencies, states, and indirections.

## Engineering Priorities

Resolve tradeoffs in this order:

1. Protect user data and restore the terminal.
2. Reject unsafe or excessive input predictably.
3. Preserve correct document meaning and logical positions.
4. Keep behavior understandable and testable.
5. Meet measured responsiveness and memory budgets.
6. Reduce implementation complexity.

Performance never justifies silent corruption, unbounded allocation, broken
terminal cleanup, or undocumented unsafe code.

## Rust Baseline

- The project must compile on the minimum supported stable Rust version recorded
  in `Cargo.toml` and on the current stable toolchain used by CI.
- `Cargo.lock` must remain committed and normal CI or release commands must use
  `--locked`.
- The workspace must set shared Rust and Clippy lint policy in the root manifest
  rather than relying on each contributor's editor.
- `unsafe_code` must be forbidden in TermLeaf crates by default. A future need
  for unsafe code requires a documented design and security review before the
  lint policy changes.
- Warnings must fail CI. A lint suppression must be as narrow as possible and
  include a short reason when the surrounding code does not make it obvious.
- Generated code must be clearly identified and must not weaken lint policy for
  handwritten modules.

## Architecture

- `document`, `layout`, `reader`, and `persistence` must remain independent of
  Ratatui and Crossterm.
- Parsing and state transitions must be testable without a real terminal.
- UI code may translate core results into terminal cells but must not define
  durable document positions, wrapping rules, or search offsets.
- Modules should own one coherent responsibility. Split a module when it has
  independent reasons to change, not to satisfy an arbitrary line count.
- Dependency direction must follow `project_plan.md`; cycles hidden through
  callbacks, globals, or broad context objects are still architectural cycles.
- Cross-module state changes should use explicit domain operations or actions.
  Avoid public fields that allow invalid combinations to be assembled.
- Global mutable state is prohibited. Process-wide immutable constants and
  deliberate one-time initialization are acceptable.

## Types and APIs

- Represent important identities, logical positions, byte counts, dimensions,
  and limits with domain types when primitive values could be confused.
- Types must make invalid states difficult to represent. Validate invariants at
  construction or parsing boundaries rather than repeatedly downstream.
- Public APIs should expose the least capability needed and keep fields private
  unless direct mutation cannot violate an invariant.
- Prefer borrowing over cloning, but do not add complex lifetimes solely to
  avoid small, measured-unimportant copies.
- Return iterators or slices when callers do not need ownership. Avoid returning
  internal containers when that commits the API to an implementation detail.
- Use checked or saturating arithmetic deliberately for reader-controlled sizes,
  offsets, and counts. Never rely on release-mode integer wrapping.
- Units belong in names or types, such as bytes, cells, pixels, rows, and
  durations. Conversions must be explicit and checked.
- Boolean parameters that obscure meaning should become a small enum or an
  options type. Do not create a builder for a handful of stable arguments.
- Public behavior must be deterministic unless time, randomness, or platform
  behavior is an explicit input.

## Errors and Panics

- Expected failures must use typed domain errors that callers can match.
- Add path, resource, or operation context at I/O and application boundaries
  without exposing sensitive unrelated paths.
- Production code must not use `unwrap`, `expect`, `panic!`, `todo!`, or
  `unreachable!` on any path reachable through user input, files, terminal
  events, configuration, or persisted state.
- An assertion or panic is acceptable only for a genuine internal invariant
  whose violation is a programming defect. Its message must identify the
  invariant, and a test should exercise the surrounding logic.
- Errors must preserve the previous valid state whenever an operation cannot
  complete atomically.
- Reader-facing errors must say what failed, why, and what action is possible.
  Debug formatting and backtraces must not leak into the alternate screen.
- Ignoring a `Result` requires an explicit reason. Cleanup should attempt every
  restoration step and retain the most useful failure information.

## Untrusted Input

- TXT, Markdown, EPUB, XHTML, XML, SVG, images, URLs, configuration, and saved
  state are untrusted input.
- Enforce limits before allocation where possible and while reading when
  metadata can lie. Limits must cover bytes, entries, dimensions, depth, nodes,
  ratios, work items, and elapsed work where relevant.
- Archive paths must never escape the archive namespace. Do not extract books
  to disk as part of normal reading.
- Parsing must not execute scripts, resolve external entities, access host files
  referenced by a book, or perform network requests.
- A malformed input must return a bounded error, not panic, loop indefinitely,
  exhaust memory, or leave partially trusted state behind.
- Security-sensitive checks should live at TermLeaf's boundary even when a
  dependency performs similar validation.
- Every new parser or decoder boundary must include deterministic adversarial,
  exact-boundary, seeded-property, hostile-corpus, and fixed-mutation coverage
  appropriate to its risk. Record whether optional fuzz discovery is warranted.

## Terminal Safety

- Terminal setup and restoration must use an owned guard whose cleanup runs on
  every ordinary return path.
- Normal exit, startup failure, handled error, Ctrl-C, and recoverable panic
  paths must restore raw mode, cursor visibility, alternate screen, mouse mode,
  paste mode, and keyboard enhancements that TermLeaf changed.
- Core logic must never write escape sequences directly. Terminal protocol
  output belongs behind one narrow boundary.
- Rendering must be a pure projection of application state wherever practical.
- Input handling must be mode-aware. Paste and text events must not reach modes
  that do not accept text.
- Tests must prove cleanup behavior; a `Drop` implementation alone is not
  sufficient evidence.

## Concurrency

- Use synchronous code until work demonstrably needs a worker thread.
- Worker queues and result channels must be bounded. Every task must have an
  owner, a completion path, and a shutdown policy.
- Generation identifiers or equivalent state must prevent stale parsing,
  layout, search, or image results from replacing current results.
- Do not hold locks while performing file I/O, decoding, rendering, channel
  sends, or other potentially blocking work.
- Shared mutable state should be smaller than the operation using it. Message
  passing is preferred when ownership can move cleanly.
- Thread tests must use deterministic coordination and timeouts, not arbitrary
  sleeps.
- A worker failure must become an application error or fallback, never a silent
  permanently pending state.

## Persistence and Privacy

- Source books must remain immutable unless a future product decision explicitly
  introduces editing.
- State writes must use the atomic same-directory sequence defined in
  `project_plan.md`; failed writes must retain the previous valid file.
- On-disk structures must include a schema version from their first release.
- Schema changes require migration, downgrade or rejection behavior, corruption
  tests, and documentation.
- Persist only data needed for reader-visible features. Do not add telemetry,
  identifiers, library scanning, network access, or diagnostic uploads without
  an explicit product and privacy decision.
- Logs and errors must avoid book contents, note text, search history, and full
  paths unless that detail is necessary and the reader requested diagnostics.
- Tests that touch platform directories must redirect them to isolated temporary
  directories.

## Performance

- Meet the budgets in `project_plan.md` with representative books before making
  public performance claims.
- Optimize measured bottlenecks, not imagined ones. Record the fixture, machine,
  profile, and before-and-after result for a performance-driven change.
- Avoid whole-book copies in layout, navigation, and search paths when slices,
  ranges, or section-level work are sufficient.
- Caches must have an owner, bounded growth, an invalidation rule, and a test for
  stale results.
- Work that can exceed one interactive frame should be interruptible or moved
  off the UI thread without weakening resource limits.
- Benchmark code must use release-equivalent optimization and must not turn
  noisy shared-runner timing into a hard correctness gate.

## Dependencies

- Add a crate only when it removes meaningful risk or maintenance work that is
  not clearer to implement locally.
- Before adding or upgrading a dependency, review maintenance, license, source,
  default features, transitive graph, platform impact, unsafe usage, and known
  advisories.
- Disable default features when they add unused formats, native libraries,
  runtimes, network clients, or license obligations.
- Production dependencies must not be added solely to simplify tests.
- Duplicate major versions and large graph increases require a recorded reason.
- Lockfile changes must be reviewed as code changes, including unexpected new
  build scripts or native dependencies.

## Style and Documentation

- `rustfmt` defines formatting. Do not hand-align code against it.
- Follow Rust naming conventions and use domain language from the project plan.
- Functions should do one understandable job. Extract helpers when they clarify
  an invariant, remove real duplication, or isolate a testable decision.
- Prefer straightforward control flow. Early returns are encouraged when they
  keep the successful path visible.
- Comments explain constraints, safety, or non-obvious reasons. They should not
  narrate syntax or preserve obsolete implementation history.
- Public APIs and non-obvious invariants need rustdoc. Examples must compile as
  doctests when practical.
- UI changes must preserve the hierarchy, focus, responsive, accessibility, and
  safety intent in `ui_mockups.md`, or update that specification and mapped test
  cases in the same change.
- `TODO` and `FIXME` comments must state a concrete missing condition and link
  to tracked work when they can outlive the current change.
- Do not leave commented-out code, debug prints, placeholder branches, or broad
  lint allowances in merged code.

## Testing

- Every reader-visible behavior needs tests at the lowest useful layer and at
  each risky boundary it crosses.
- Tests should use arrange, act, and assert structure without comments when the
  code already reads clearly.
- Test names must describe behavior and conditions, not implementation method
  names alone.
- Unit tests cover invariants and edge cases. Integration tests cover module,
  filesystem, process, and terminal boundaries.
- Property tests cover layout, navigation, offsets, serialization, progress,
  and untrusted-boundary invariants. Deterministic malformed-input, hostile
  corpus, and fixed-mutation suites cover parsing and state loading; optional
  fuzz targets provide additional discovery rather than required gate evidence.
- Regression tests must fail before the fix and explain the original failure in
  their name or a short comment.
- Tests must be deterministic, isolated from user state and the network, and
  safe to run in parallel unless explicitly placed in a low-parallelism suite.
- Time, locale, terminal capabilities, dimensions, paths, and environment must
  be injected or fixed when they affect an assertion.
- Snapshot tests require direct invariant assertions and deliberate review. A
  snapshot update is not proof that behavior is correct.
- Test fixtures must have provenance and licensing recorded. Small synthetic
  fixtures are preferred for focused behavior; ignored full books support
  realistic local journeys.
- Security limit tests must exercise the boundary, one value below it where
  meaningful, and one value above it.
- Code is not complete when only the happy path passes.
- Every implementation change must select stable case IDs and executable
  profiles from `testcases.md`; broad claims such as "tests passed" are not
  sufficient evidence.

## Required Validation

Every Rust change must pass the applicable commands from the repository root:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --doc --locked
cargo deny check
```

Run narrower tests while developing, then run the complete applicable cycle
before considering the change finished. Platform, PTY, benchmark, and release
checks run when the affected risk or delivery gate requires them. Optional fuzz
runs execute only when explicitly selected for scheduled or pre-release work.

Record exact commands, outcomes, skipped checks, environment, and fixtures in
`testreport.md` for every commit. After the complete local Rust validation cycle,
run `cargo clean` and record its result. Never report a command as passing when
it did not run.

## Review Checklist

A change is ready only when the reviewer can answer yes to each applicable item:

- Does the behavior match the product contract and module boundaries?
- Are invalid states, units, ownership, and limits visible in the types?
- Can reader-controlled input fail without panic or excessive resource use?
- Are terminal and persisted state preserved after every failure path?
- Are errors useful without exposing unrelated private data?
- Are concurrency, caches, and background work bounded and cancellable?
- Are dependencies and feature flags justified?
- Do tests cover success, boundaries, failures, and the original regression?
- Did all applicable validation run and get recorded honestly?
- Does documentation and tracking match the resulting behavior?
- Was `cargo clean` run after the completed local Rust validation cycle?

## Exceptions

A standard may be relaxed only when following it would make TermLeaf less safe,
correct, maintainable, or portable. Record the exception beside the code when
local, and in `commit_tracker.md` when it affects architecture or future work.
Include the reason, scope, risk, compensating tests, and condition for removing
the exception. Convenience or schedule pressure alone is not sufficient.
