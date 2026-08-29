# Tasks Left

This is the ordered implementation queue for the first release. Task numbers
remain stable; completed tasks should be struck through rather than removed.
“Implement the next 10” means the first ten incomplete tasks from the top. Each
batch must preserve dependency order and finish its listed acceptance criteria.

Coverage-guided fuzzing is optional because sustained fuzz campaigns consume
substantial time, compute, output, and disk space. Required robustness evidence
uses deterministic malformed-input tables, exact boundary cases, fixed-seed
properties, curated hostile fixtures, and small fixed mutation suites. Any bug
found by optional fuzzing must become a deterministic regression test.

## Phase 0

No implementation tasks remain. The Rust foundation gate is complete.

## Phase 1

No implementation tasks remain. The plain-text reading-loop gate is complete.

## Phase 2

1. ~~**Reconcile the deterministic test policy and manifests.** Update the registry generator and regenerate profile, case-registry, and phase-gate manifests so deterministic robustness cases are mandatory and `FUZZ-*` durations are optional.~~
2. ~~**Build the structured hostile fixture corpus.** Commit or deterministically generate registered TXT, Markdown, malformed EPUB, raster, SVG, and SVGZ fixtures with hashes, provenance, licenses, parameters, and served case IDs.~~
3. ~~**Harden plain-text decoding deterministically.** Add malformed encodings, exact byte limits, fixed-seed properties, hostile fixtures, and bounded mutations that always produce a valid model or typed bounded error.~~
4. **Harden ZIP parsing and archive names deterministically.** Test malformed headers and paths, exact count and size limits, checked arithmetic, canonicalization properties, collisions, and fixed archive mutations without extraction or escape.
5. **Harden EPUB control parsing deterministically.** Test malformed container, OPF, NCX, and navigation documents; structural limits; fixed-seed properties; hostile fixtures; XML mutations; and denial of external resolution.
6. **Harden XHTML conversion deterministically.** Test malformed, deep, and wide XHTML; source-range invariants; active-content rejection; and fixed tag, entity, and attribute mutations.
7. **Harden Markdown conversion deterministically.** Test malformed event streams, byte and work limits, valid-model properties, hostile raw HTML, and fixed delimiter and offset mutations.
8. **Harden SVG and SVGZ processing deterministically.** Test resolver denial, compressed and decompressed limits, geometry, work, allocation, hostile fixtures, and fixed SVG/XML mutations.
9. **Harden raster decoding deterministically.** Test malformed and truncated data for every enabled decoder, exact dimension and allocation limits, licensed fixtures, and small fixed header and body mutations.
10. **Complete structured wide and narrow render evidence.** Pass `MD-003`, `MD-007`, `MD-011`, `EPUB-012`, and `EPUB-013` with direct assertions for semantics, source order, code, tables, images, and responsive placement.
11. **Implement native graphics transports.** Add bounded Kitty, Sixel, and iTerm2 output with chunking, identifiers, replacement, deletion, resize, navigation, and shutdown cleanup instead of caption-only fallback.
12. **Implement terminal graphics capability probing.** Complete `IMG-017` with bounded queries and timeouts that accept only positive evidence and never emit a competing protocol after malformed, delayed, absent, or spoofed responses.
13. **Prove image lifecycle behavior over PTY.** Add hosted PTY journeys for display, replacement, scrolling, fallback, cancellation, worker shutdown, stale-image cleanup, terminal restoration, and relevant `TERM-009`, `IMG-017`, and `IMG-018` behavior.
14. **Write and execute the `IMG-018` native procedure.** Record display, resize, scroll, delete, failure, and exit results on at least one finalized terminal for every protocol TermLeaf claims.
15. **Run the complete Phase 2 hosted matrix.** At one pushed revision, retain Linux, macOS, and Windows evidence for registry, core, render, security, MSRV, dependency, and native PTY profiles.
16. **Close the Phase 2 gate.** Promote only evidenced cases, record skips and native limits, regenerate gate files, and update the README, tracker, test report, and procedures without claiming optional fuzz runs.

## Phase 3

17. **Resolve non-TTY behavior.** Decide `DEC-TEST-002` and specify exact piped input and output behavior for `CLI-009` and `A11Y-005`.
18. **Freeze text and persisted-state limits.** Resolve `DEC-TEST-012` for paths, paste, queries, notes, URLs, recents, annotations, configuration, state sizes, nesting, entries, and total persisted data.
19. **Resolve configuration error behavior.** Settle `DEC-TEST-005`, including unknown keys, invalid types, syntax failures, inaccessible files, startup fallback, and typed diagnostics.
20. **Resolve search semantics.** Settle `DEC-TEST-004`, including direction start points, block boundaries, normalization, wrapping, history capacity, and control-character queries.
21. **Define document identity and relocation.** Resolve `DEC-TEST-006` with privacy-conscious identity evidence and exact moved, missing, edited, and stale-anchor outcomes.
22. **Define state-writer concurrency.** Resolve `DEC-TEST-007` with locking or linearization, stale-owner recovery, migration races, and last-writer behavior.
23. **Freeze remaining Phase 3 interactions.** Decide help navigation, temporary-view return stacking, selection keys and cross-block ranges, note-save keys, and versioned accessible highlight identifiers.
24. **Complete the configuration schema.** Implement typed loading, platform-native paths, limits, precedence, warnings, errors, and `CFG-001` through `CFG-004` without rewriting user TOML.
25. **Implement versioned state models.** Add exact schemas and round trips for positions, recents, search history, annotations, settings, current, old, and future versions, plus deterministic migrations.
26. **Implement atomic durable state writes.** Use same-directory temporary files, the chosen sync and replacement policy, cleanup, private permissions, and fault injection while preserving the prior valid state.
27. **Secure concurrent state storage.** Reject unsafe destination types and swaps, enforce all limits, implement the chosen writer policy, and pass `STATE-009` and `STATE-011` through `STATE-015`.
28. **Restore and checkpoint reading positions.** Save per-book logical anchors and restore them across resize, mode, theme, normal exit, supported interruption, and stale-anchor recovery.
29. **Implement recent books.** Deliver bounded MRU reopen, remove, and clear behavior; trustworthy metadata fallback; stale entries; current position; responsive views; and no directory scanning.
30. **Implement Open Path.** Replace the placeholder with isolated text focus, bounded typing and paste, validation recovery, responsive layout, no scanning, and exact return behavior.
31. **Implement the search engine.** Deliver literal smart-case forward and backward search, Unicode normalization mapping, block policy, exact ranges, generation cancellation, and all core `SEARCH` cases.
32. **Implement search UI and history.** Add entry, results, non-color match cues, counts, wrap messages, edit and no-result states, bounded local history, removal, and confirmed clearing.
33. **Implement text selection.** Add grapheme-safe start, extension, reversal, cancellation, wrapping and resize survival, and exact highlight or note attachment ranges.
34. **Implement bookmarks.** Support creation, naming, renaming, listing, jumping, return behavior, deletion, persistence, validation, and exact intermediate states.
35. **Implement highlights.** Support valid logical ranges, accessible versioned colors, non-color cues, relayout stability, persistence, editing, and deletion.
36. **Implement notes.** Add point and range attachment, bounded plain-text editing and paste, multiline round trips, cancellation, save-failure recovery, and terminal-safe rendering.
37. **Implement annotation management.** Deliver current-book filtering, previews, context, stable ordering, jump, edit, delete, unresolved states, narrow layout, and destructive confirmation.
38. **Complete help and feedback views.** Replace placeholders with contextual help, a status glossary, theme details, recoverable errors, save-pending and save-failed states, and registry-derived bindings.
39. **Harden state, configuration, and actions deterministically.** Replace required `FUZZ-008` through `FUZZ-010` intent with malformed and boundary tests, fixed-seed properties, hostile corpora, and bounded mutation and action-sequence suites.
40. **Pass the Phase 3 native and accessibility gate.** Write missing procedures and record `KEY-005`, text-entry isolation, keyboard-only, non-color, redraw, locale, screen-reader, and hosted matrix evidence.

## Phase 4

41. **Freeze the external-link policy.** Resolve `DEC-TEST-008` for schemes, URL length, normalization, suspicious syntax, display escaping, and launcher arguments.
42. **Implement external-link focus and confirmation.** Deliver logical traversal, internal navigation, unsupported states, full scrollable destination display, confirmation, cancellation, and single activation.
43. **Implement safe platform launchers.** Pass one validated URL as one non-shell argument and handle a missing launcher, spawn error, timeout, and child failure on every supported OS.
44. **Refine annotation relocation.** Apply fixed source edits and moves to every annotation type, relocate only with approved evidence, and retain safe unresolved items otherwise.
45. **Refine metadata and critical-value inspection.** Complete recent and book details, recognizable fallbacks, long wrapped paths, URLs and diagnostics, narrow layouts, and return-stack behavior.
46. **Complete the privacy audit.** Prove there is no networking, scanning, unrelated reading, source rewriting, sidecar creation, undocumented storage, telemetry identifier, or private diagnostic leakage.
47. **Complete Paper and accessibility matrices.** Test every color capability, viewport, focus, search, selection, annotation, warning, image, loading, and error state with direct non-color and contrast assertions.
48. **Build the benchmark framework.** Register representative hardware, fixtures, markers, sampling, memory methods, thresholds, and deterministic release-profile harnesses for `PERF-001` through `PERF-009`.
49. **Meet performance and retention budgets.** Measure and optimize launch, open, navigation, relayout, save, memory, hostile cancellation, search scaling, and cache steady state, or document narrow exceptions.
50. **Run usability sessions and finish guides.** Test common journeys with readers other than the author and revise user, troubleshooting, accessibility, privacy, and contributor guidance from recorded findings.
51. **Optionally add coverage-guided fuzzing.** Only if time and compute permit, add bounded `cargo-fuzz` targets and corpora for scheduled or pre-release discovery, retain crashes, and promote each defect to a deterministic regression.

## Phase 5

52. **Finalize support and packaging policy.** Select promised OS versions, architectures, terminals, sessions, image protocols, package channels, deferred combinations, and evidence owners.
53. **Configure cargo-dist native artifacts.** Produce locked native archives or installers with versioned manifests, checksums, source references, and traceability to a protected tag.
54. **Harden release automation.** Pin actions, minimize permissions, restrict artifact origins to protected tags and commits, and validate workflow identity and package contents.
55. **Complete dependency and notice manifests.** Review every enabled image decoder and dependency, generate third-party notices, and reconcile Cargo and non-Cargo asset provenance.
56. **Test clean installation and first run.** Follow published instructions on clean supported accounts and prove installation, help and version output, fixture opening, reading, and clean exit.
57. **Test upgrade and rollback disposition.** Run migration and rollback against a named predecessor or record first-release non-applicability with a future procedure.
58. **Run the final native terminal matrix.** Execute build, core, doctests, PTY lifecycle, install, reading, search, annotation, error, help, resize, SSH or tmux, image fallback, restoration, and exit on every claimed tuple.
59. **Publish captures and known limitations.** Record representative delivered UI and precise Unicode, bidi, image, terminal, screen-reader, and accessibility boundaries supported by evidence.
60. **Execute the release and rollback checklist.** Pass every cumulative gate, P0 and P1 case, required environment, artifact, checksum, provenance, notice, clean-install, and release-manifest check before tagging.
