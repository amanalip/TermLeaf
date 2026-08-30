# Tasks Left

This is the ordered implementation queue for the first release. Milestone
numbers remain stable; completed milestones and work units should be struck
through rather than removed. Lettered work units are sized so one contributor
can usually finish one, including focused tests and documentation, within a
five-hour work window. They are estimates, not deadlines: split a unit before
starting if investigation reveals materially larger scope.

“Implement the next 10” means the first ten incomplete lettered work units from
the top. Work units under the same milestone run in listed order unless their
text explicitly permits parallel work. A milestone is complete only when all of
its required work units are complete.

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
4. ~~**Harden ZIP parsing and archive names deterministically.** Test malformed headers and paths, exact count and size limits, checked arithmetic, canonicalization properties, collisions, and fixed archive mutations without extraction or escape.~~
5. ~~**Harden EPUB control parsing deterministically.** Test malformed container, OPF, NCX, and navigation documents; structural limits; fixed-seed properties; hostile fixtures; XML mutations; and denial of external resolution.~~
6. ~~**Harden XHTML conversion deterministically.** Test malformed, deep, and wide XHTML; source-range invariants; active-content rejection; and fixed tag, entity, and attribute mutations.~~
7. ~~**Harden Markdown conversion deterministically.** Test malformed event streams, byte and work limits, valid-model properties, hostile raw HTML, and fixed delimiter and offset mutations.~~
8. ~~**Harden SVG and SVGZ processing deterministically.** Test resolver denial, compressed and decompressed limits, geometry, work, allocation, hostile fixtures, and fixed SVG/XML mutations.~~
9. ~~**Harden raster decoding deterministically.** Test malformed and truncated data for every enabled decoder, exact dimension and allocation limits, licensed fixtures, and small fixed header and body mutations.~~
10. ~~**Complete structured wide and narrow render evidence.** Pass `MD-003`, `MD-007`, `MD-011`, `EPUB-012`, and `EPUB-013` with direct assertions for semantics, source order, code, tables, images, and responsive placement.~~
11. ~~**Implement native graphics transports.** Add bounded Kitty, Sixel, and iTerm2 output with chunking, identifiers, replacement, deletion, resize, navigation, and shutdown cleanup instead of caption-only fallback.~~
12. **Implement terminal graphics capability probing.**
    - [ ] **12a. Freeze query policy.** Specify query bytes, response limits, timeout values, parser states, precedence, and fallback behavior for Kitty, Sixel, and iTerm2 without changing case status.
    - [ ] **12b. Build the bounded probe transport.** Send queries and collect partial responses under byte and time limits while preserving unrelated input for the event loop.
    - [ ] **12c. Parse positive capability evidence.** Accept only protocol-specific complete responses and reject malformed, partial, delayed, absent, and spoofed values with typed outcomes.
    - [ ] **12d. Integrate one-shot backend selection.** Apply locked override and protocol precedence once, emit no competing protocol, and retain cell/caption fallback.
    - [ ] **12e. Complete deterministic `IMG-017` evidence.** Add response tables and timeout tests, register locations, and record exact local checks without claiming hosted PTY behavior.
13. **Prove image lifecycle behavior over PTY.**
    - [ ] **13a. Extend the PTY harness for image protocols.** Capture probe and graphics bytes with deterministic deadlines, dimensions, child cleanup, and protocol-aware assertions.
    - [ ] **13b. Prove display, replacement, and scrolling.** Exercise native placement, unchanged frames, replacement IDs, viewport entry/exit, and stale-image deletion.
    - [ ] **13c. Prove fallback and cancellation.** Exercise absent/malformed capability responses, missing geometry, decode or encode failure, navigation cancellation, and current-generation recovery.
    - [ ] **13d. Prove shutdown and restoration.** Terminate during queued and active image work and assert bounded joins, cleanup attempts, terminal restoration, and no surviving child or worker.
    - [ ] **13e. Register PTY lifecycle evidence.** Map the journeys to `TERM-009`, `IMG-017`, and supporting `IMG-018` locations while leaving manual acceptance Planned.
14. **Write and execute the `IMG-018` native procedure.**
    - [ ] **14a. Write the protocol-neutral procedure.** Define prerequisites, fixture, dimensions, display, resize, scroll, replacement, failure, exit, capture, and cleanup observations.
    - [ ] **14b. Execute and record Kitty acceptance.** Run the procedure on one finalized claimed Kitty terminal and retain terminal/version evidence and results.
    - [ ] **14c. Execute and record Sixel acceptance.** Run the procedure on one finalized claimed Sixel terminal and retain geometry, terminal/version evidence, and results.
    - [ ] **14d. Execute and record iTerm2 acceptance.** Run the procedure on one finalized claimed iTerm2 terminal and retain terminal/version evidence and results.
    - [ ] **14e. Reconcile native claims.** Mark only evidenced protocol tuples accepted, document exclusions precisely, and update `IMG-018` status and links.
15. **Run the complete Phase 2 hosted matrix.**
    - [ ] **15a. Prepare one gate revision.** Ensure a clean pushed revision, fresh manifests, deterministic fixtures, and exact profile commands before triggering hosted jobs.
    - [ ] **15b. Run Linux profiles.** Retain registry, core, render, security, dependency, MSRV, and native PTY outcomes with run URLs and revision.
    - [ ] **15c. Run macOS profiles.** Retain registry, core, render, security, dependency, MSRV, and native PTY outcomes with run URLs and revision.
    - [ ] **15d. Run Windows profiles.** Retain registry, core, render, security, dependency, MSRV, and ConPTY outcomes with run URLs and revision.
    - [ ] **15e. Resolve hosted-only failures.** Convert defects to deterministic regressions, fix them, and rerun the complete affected matrix at one new pushed revision.
    - [ ] **15f. Record the final hosted matrix.** Map every required environment row and profile to the passing revision without combining evidence from incompatible revisions.
16. **Close the Phase 2 gate.**
    - [ ] **16a. Audit case status.** Compare every Phase 2 case with executable, hosted, and manual evidence; preserve Planned or Blocked status where evidence is incomplete.
    - [ ] **16b. Regenerate and verify gate artifacts.** Refresh case, profile, fixture, and phase-gate manifests and run every local gate command.
    - [ ] **16c. Document limits and skips.** Record native-terminal boundaries, unsupported combinations, optional fuzz non-runs, and residual risks.
    - [ ] **16d. Update release-facing documents.** Reconcile README, implementation and commit trackers, test report, and manual procedures against the audited status.
    - [ ] **16e. Commit and push Phase 2 closure.** Review the complete diff, create logical commits, push the gate revision, and verify the branch and hosted links are synchronized.

## Phase 3

17. **Resolve non-TTY behavior.**
    - [ ] **17a. Inventory non-TTY entry paths.** Record current stdin/stdout/stderr and terminal-initialization behavior for files, pipes, redirection, and unsupported combinations.
    - [ ] **17b. Decide `DEC-TEST-002`.** Freeze exact piped-input and redirected-output behavior, exit codes, diagnostics, limits, and accessibility implications.
    - [ ] **17c. Update `CLI-009` and `A11Y-005`.** Reconcile catalog, procedures, manifests, and implementation prerequisites with the decision.
18. **Freeze text and persisted-state limits.**
    - [ ] **18a. Inventory bounded text surfaces.** List paths, paste, queries, notes, URLs, names, metadata, diagnostics, and configuration strings with current implicit limits.
    - [ ] **18b. Freeze text-input limits.** Set byte, scalar, grapheme, line, and control-character policies for interactive and imported text.
    - [ ] **18c. Freeze collection and nesting limits.** Set capacities for recents, history, annotations, configuration, versions, nesting, entries, and total persisted bytes.
    - [ ] **18d. Record `DEC-TEST-012`.** Update cases and design documents with inclusive boundaries, rejection order, and recovery behavior.
19. **Resolve configuration error behavior.**
    - [ ] **19a. Define syntax and type failures.** Freeze outcomes for malformed TOML, invalid values, unknown keys, and unsupported versions.
    - [ ] **19b. Define I/O and precedence failures.** Freeze outcomes for missing, unreadable, unsafe, and platform-path failures plus CLI/config/default precedence.
    - [ ] **19c. Record `DEC-TEST-005`.** Specify typed diagnostics, startup fallback, rewrite prohibition, and exact case updates.
20. **Resolve search semantics.**
    - [ ] **20a. Freeze matching semantics.** Decide literal smart-case behavior, Unicode normalization, grapheme boundaries, block crossing, and control characters.
    - [ ] **20b. Freeze navigation semantics.** Decide forward/backward start points, current-match behavior, wrapping, result ordering, and resize effects.
    - [ ] **20c. Freeze query/history semantics.** Decide query limits, empty input, deduplication, capacity, persistence, removal, and clearing.
    - [ ] **20d. Record `DEC-TEST-004`.** Update cases and design documents with examples and exact expected ranges.
21. **Define document identity and relocation.**
    - [ ] **21a. Evaluate identity evidence.** Compare canonical path, metadata, content fingerprints, and privacy/storage costs against required move and edit behavior.
    - [ ] **21b. Freeze identity and stale-book outcomes.** Decide moved, missing, replaced, edited, duplicate, and inaccessible source behavior.
    - [ ] **21c. Freeze anchor relocation evidence.** Define exact-match and contextual evidence, ambiguity handling, and unresolved preservation.
    - [ ] **21d. Record `DEC-TEST-006`.** Update schemas, cases, privacy claims, and migration prerequisites.
22. **Define state-writer concurrency.**
    - [ ] **22a. Choose writer linearization.** Compare locking and optimistic replacement and freeze ownership, timeout, and same-process behavior.
    - [ ] **22b. Define recovery and races.** Specify stale-owner recovery, migration races, concurrent updates, and last-writer outcomes.
    - [ ] **22c. Record `DEC-TEST-007`.** Update state cases, fault matrix, platform assumptions, and implementation contract.
23. **Freeze remaining Phase 3 interactions.**
    - [ ] **23a. Freeze help and temporary-view navigation.** Decide invocation scope, return stacking, nested temporary views, focus restoration, and cancellation.
    - [ ] **23b. Freeze selection interactions.** Decide keys, anchor/caret behavior, reversal, cross-block ranges, resize, and cancellation.
    - [ ] **23c. Freeze note editing interactions.** Decide entry, multiline keys, save/cancel, paste, limits, and failure recovery.
    - [ ] **23d. Freeze highlight identifiers.** Define versioned colors, non-color labels, migration, contrast, and screen-reader names.
24. **Complete the configuration schema.**
    - [ ] **24a. Define typed configuration models.** Implement defaults, deserialization, validation, version handling, and exact field limits.
    - [ ] **24b. Resolve platform-native paths.** Implement explicit and default path discovery without scanning or creating unrelated files.
    - [ ] **24c. Implement precedence and diagnostics.** Apply CLI over config over defaults and surface frozen warnings or typed errors without rewriting TOML.
    - [ ] **24d. Complete `CFG-001` through `CFG-004`.** Add boundary, malformed, unknown-key, precedence, and no-rewrite evidence and register it.
25. **Implement versioned state models.**
    - [ ] **25a. Define current state envelope.** Implement version, document identity, settings, and bounded top-level collections.
    - [ ] **25b. Implement position and recent models.** Add validated logical anchors, metadata snapshots, ordering fields, and round trips.
    - [ ] **25c. Implement search and annotation models.** Add bounded history, bookmarks, highlights, notes, identifiers, and unresolved states.
    - [ ] **25d. Implement migration dispatch.** Decode current and supported old versions deterministically and reject future versions without data loss.
    - [ ] **25e. Add schema round-trip evidence.** Cover empty, maximum, old, current, future, malformed, and cross-field-invalid states.
26. **Implement atomic durable state writes.**
    - [ ] **26a. Implement same-directory temporary writes.** Create private temporary state beside the destination and serialize under the frozen size limit.
    - [ ] **26b. Implement sync and replacement.** Apply the chosen file and directory sync order and platform replacement policy.
    - [ ] **26c. Implement cleanup and permissions.** Remove abandoned temporaries where safe and verify private permissions on creation and replacement.
    - [ ] **26d. Add write fault injection.** Fail each open, write, flush, sync, rename, and cleanup step while preserving the prior valid state.
27. **Secure concurrent state storage.**
    - [ ] **27a. Reject unsafe state paths.** Deny symlinks, non-regular files, ownership or permission violations, and path swaps at each opening boundary.
    - [ ] **27b. Implement writer coordination.** Apply the chosen lock or linearization policy with bounded wait and stale-owner recovery.
    - [ ] **27c. Enforce aggregate state limits.** Reject excessive bytes, nesting, entries, text, and collections before unbounded allocation.
    - [ ] **27d. Complete `STATE-009` and `STATE-011` through `STATE-015`.** Add deterministic race, swap, limit, fault, and recovery evidence.
28. **Restore and checkpoint reading positions.**
    - [ ] **28a. Persist logical reading anchors.** Save bounded per-book section, block, source offset, mode, and contextual relocation evidence.
    - [ ] **28b. Restore exact anchors.** Reopen unchanged books at the same logical passage across viewport, theme, and mode differences.
    - [ ] **28c. Recover stale anchors.** Apply the frozen relocation policy for edited, moved, missing, or ambiguous books and retain unresolved state safely.
    - [ ] **28d. Implement checkpoint triggers.** Save on bounded cadence, navigation milestones, normal exit, and supported interruption without blocking terminal cleanup.
    - [ ] **28e. Add position lifecycle journeys.** Cover resize, mode, theme, crash-safe prior state, interruption, and stale recovery.
29. **Implement recent books.**
    - [ ] **29a. Implement bounded MRU updates.** Insert, deduplicate, reorder, cap, and persist recents using frozen document identity.
    - [ ] **29b. Implement metadata and stale states.** Show trustworthy title/path fallback, current position, missing or inaccessible markers, and no directory scanning.
    - [ ] **29c. Implement recent-book actions.** Reopen, remove, clear with confirmation, and preserve selection and return behavior.
    - [ ] **29d. Build responsive recent views.** Render wide through narrow states with long safe paths and non-color focus cues.
    - [ ] **29e. Complete recent-book evidence.** Add ordering, capacity, stale, persistence, UI, and privacy tests.
30. **Implement Open Path.**
    - [ ] **30a. Build isolated text focus.** Route bounded typing, deletion, movement, paste, submit, and cancel without triggering reader bindings.
    - [ ] **30b. Validate submitted paths.** Apply frozen text/path limits and supported-source checks without scanning or rewriting input.
    - [ ] **30c. Implement success and recovery flow.** Open valid books, preserve editable invalid input, display typed diagnostics, and restore the invoking view exactly.
    - [ ] **30d. Build responsive Open Path UI.** Render focus, help, validation, long input, paste, and narrow states accessibly.
    - [ ] **30e. Complete Open Path evidence.** Add keyboard, paste, limit, no-scan, error recovery, and return-stack tests.
31. **Implement the search engine.**
    - [ ] **31a. Build normalized searchable text mapping.** Map normalized comparison text back to exact source and grapheme-safe logical ranges.
    - [ ] **31b. Implement literal smart-case matching.** Cover forward and backward matching, block policy, controls, and exact boundary semantics.
    - [ ] **31c. Implement result navigation.** Apply start-point, current-match, wrap, ordering, and end-state rules.
    - [ ] **31d. Add generation cancellation and limits.** Bound query, work, results, memory, stale completion, and navigation-away behavior.
    - [ ] **31e. Complete core `SEARCH` evidence.** Add normalization, direction, boundaries, wrapping, cancellation, and fixed-seed properties.
32. **Implement search UI and history.**
    - [ ] **32a. Build isolated search entry.** Support bounded edit and paste, submit, cancel, direction, and invoking-view restoration.
    - [ ] **32b. Render search results and feedback.** Show current/total, wrap, no-result, invalid, loading, and non-color match cues.
    - [ ] **32c. Implement local search history.** Deduplicate, cap, navigate, persist, remove, and confirm clearing under frozen semantics.
    - [ ] **32d. Preserve search through layout changes.** Keep logical matches and current result stable across resize, theme, and mode changes.
    - [ ] **32e. Complete search UI evidence.** Add entry isolation, feedback, history, responsive, and accessibility journeys.
33. **Implement text selection.**
    - [ ] **33a. Define logical selection state.** Store anchor and caret as validated grapheme-safe document positions independent of visual rows.
    - [ ] **33b. Implement selection movement.** Start, extend, contract, reverse, cross allowed block boundaries, and cancel using frozen keys.
    - [ ] **33c. Render accessible selection.** Add theme and non-color cues without altering source text or terminal cell width.
    - [ ] **33d. Preserve selection through relayout.** Keep exact logical ranges across wrapping, resize, theme, and mode changes.
    - [ ] **33e. Expose annotation attachment ranges.** Convert valid selections into bookmark/highlight/note inputs and reject unsupported ranges.
34. **Implement bookmarks.**
    - [ ] **34a. Implement bookmark creation and validation.** Create point annotations with bounded optional names and stable document identity.
    - [ ] **34b. Implement bookmark list and jump.** Filter current book, order stably, preview context, jump, and restore return state.
    - [ ] **34c. Implement rename and delete.** Preserve editable failures, confirm deletion, and handle unresolved bookmarks.
    - [ ] **34d. Persist and migrate bookmarks.** Round-trip current records and supported old forms under limits.
    - [ ] **34e. Complete bookmark evidence.** Cover intermediate UI, duplicate names, limits, relocation, persistence, and failure recovery.
35. **Implement highlights.**
    - [ ] **35a. Implement validated highlight ranges.** Create only supported nonempty logical ranges and reject cross-boundary invalid state.
    - [ ] **35b. Implement versioned highlight styles.** Store stable identifiers and map them to accessible color and non-color cues.
    - [ ] **35c. Render relayout-stable highlights.** Project logical ranges across wrapping, resize, theme, and mode without width drift.
    - [ ] **35d. Implement highlight editing and deletion.** Change style, preserve range, confirm deletion, and recover save failures.
    - [ ] **35e. Complete highlight evidence.** Cover overlap policy, persistence, migration, accessibility, relocation, and invalid ranges.
36. **Implement notes.**
    - [ ] **36a. Implement point and range note models.** Validate attachment positions, bounded text, identity, timestamps or ordering, and unresolved state.
    - [ ] **36b. Build isolated note editing.** Support multiline typing and paste, movement, save, cancel, and frozen key behavior.
    - [ ] **36c. Render terminal-safe note content.** Preserve multiline round trips while escaping controls and handling narrow layouts.
    - [ ] **36d. Implement save-failure recovery.** Keep unsaved text available, report pending/failed state, and retry or cancel without corrupting prior data.
    - [ ] **36e. Complete note evidence.** Cover limits, point/range attachments, paste, controls, persistence, relocation, and write faults.
37. **Implement annotation management.**
    - [ ] **37a. Build current-book annotation projection.** Combine bookmarks, highlights, and notes with stable ordering and unresolved markers.
    - [ ] **37b. Build responsive annotation list.** Show type, name or preview, context, focus, empty state, and narrow layouts.
    - [ ] **37c. Implement annotation navigation.** Jump to resolved items, show unresolved details, and restore the invoking view.
    - [ ] **37d. Implement edit and delete dispatch.** Open the correct editor, confirm destructive actions, and preserve selection after changes.
    - [ ] **37e. Complete management evidence.** Cover filtering, ordering, previews, jump, edit, delete, unresolved, and responsive behavior.
38. **Complete help and feedback views.**
    - [ ] **38a. Implement contextual help sections.** Generate bindings from the registry and show only actions valid for the invoking view and focus.
    - [ ] **38b. Implement status and theme reference.** Explain status fields, modes, themes, color capability, and non-color identifiers responsively.
    - [ ] **38c. Implement recoverable error views.** Show sanitized reason, recovery actions, retained input, and exact return behavior.
    - [ ] **38d. Implement persistence feedback.** Surface save-pending, saved, retryable failure, and unresolved-state messages with bounded lifetimes.
    - [ ] **38e. Complete help and feedback evidence.** Cover every view, focus, narrow class, registry drift, failure action, and return stack.
39. **Harden state, configuration, and actions deterministically.**
    - [ ] **39a. Build malformed state/configuration tables.** Cover syntax, types, versions, nesting, lengths, counts, truncation, and hostile controls.
    - [ ] **39b. Add exact boundary tests.** Exercise every frozen text, collection, allocation, file, and total-state limit at below, equal, and one over.
    - [ ] **39c. Add fixed-seed model properties.** Require arbitrary bounded state/configuration inputs to produce a valid model or typed bounded error.
    - [ ] **39d. Add bounded mutation suites.** Mutate representative current and old persisted documents and preserve prior valid state on failure.
    - [ ] **39e. Add action-sequence properties.** Exercise focus, temporary views, search, selection, annotation, save, resize, and cancellation invariants.
    - [ ] **39f. Register deterministic replacements.** Map evidence replacing required `FUZZ-008` through `FUZZ-010` intent without claiming fuzz duration.
40. **Pass the Phase 3 native and accessibility gate.**
    - [ ] **40a. Complete missing procedures.** Write exact keyboard-only, text-entry isolation, non-color, redraw, locale, and screen-reader steps and prerequisites.
    - [ ] **40b. Execute local native/accessibility journeys.** Record `KEY-005` and supported deterministic procedure halves with captures and environment identity.
    - [ ] **40c. Run the hosted Phase 3 matrix.** Execute required Linux, macOS, Windows, MSRV, dependency, render, security, and PTY profiles at one revision.
    - [ ] **40d. Record deferred human rows.** Execute available GUI/screen-reader rows and precisely mark unavailable environments without promotion.
    - [ ] **40e. Audit and close the Phase 3 gate.** Reconcile cases, manifests, reports, procedures, limits, and hosted links at the passing revision.

## Phase 4

41. **Freeze the external-link policy.**
    - [ ] **41a. Classify allowed destinations.** Decide supported schemes, internal versus external handling, and explicitly rejected syntax.
    - [ ] **41b. Freeze URL normalization and limits.** Define parsing, length, Unicode/percent encoding, suspicious forms, fragments, and display escaping.
    - [ ] **41c. Freeze launcher contract.** Define one-argument invocation, environment use, timeout, child status, and platform failure behavior.
    - [ ] **41d. Record `DEC-TEST-008`.** Update cases, privacy/security claims, and implementation examples.
42. **Implement external-link focus and confirmation.**
    - [ ] **42a. Project links into logical focus order.** Traverse links across wrapping and blocks without changing reading anchors.
    - [ ] **42b. Implement internal and unsupported outcomes.** Navigate validated internal targets and show inert typed feedback for unsupported destinations.
    - [ ] **42c. Build confirmation view.** Show the complete scrollable escaped destination, risk context, confirm, cancel, and invoking-view return.
    - [ ] **42d. Guarantee single activation.** Debounce confirmation and prevent repeated launch on redraw, key repeat, or delayed child outcome.
    - [ ] **42e. Complete link interaction evidence.** Cover focus, wrapping, internal navigation, long destinations, unsupported forms, and cancellation.
43. **Implement safe platform launchers.**
    - [ ] **43a. Implement Unix launcher selection.** Invoke one validated URL as one non-shell argument and type missing/spawn/timeout/status failures.
    - [ ] **43b. Implement macOS launcher selection.** Apply the same argument and failure contract using the supported native command.
    - [ ] **43c. Implement Windows launcher selection.** Apply the same contract without shell interpolation or argument splitting.
    - [ ] **43d. Add fake-launcher integration tests.** Capture argv and exercise success, missing binary, spawn error, timeout, and child failure deterministically.
    - [ ] **43e. Register platform evidence.** Map local tests and hosted rows without claiming platforms not executed.
44. **Refine annotation relocation.**
    - [ ] **44a. Build fixed edit fixtures.** Cover insertion, deletion, replacement, movement, duplicate context, normalization, and whole-book replacement.
    - [ ] **44b. Relocate bookmark points.** Apply only frozen identity/context evidence and preserve ambiguous or missing points unresolved.
    - [ ] **44c. Relocate highlight ranges.** Preserve exact ranges where supported and reject partial or ambiguous relocation.
    - [ ] **44d. Relocate point and range notes.** Apply the corresponding point/range policy without changing note content.
    - [ ] **44e. Complete relocation evidence.** Register deterministic outcomes for every edit fixture and annotation type.
45. **Refine metadata and critical-value inspection.**
    - [ ] **45a. Complete book details.** Show trustworthy title, author, format, source, identity, and recognizable fallback values.
    - [ ] **45b. Complete recent details.** Show current position, last-opened ordering, stale state, and safe available actions.
    - [ ] **45c. Build critical-value presentation.** Wrap full escaped paths, URLs, diagnostics, and identifiers without silent truncation.
    - [ ] **45d. Complete responsive and return behavior.** Cover wide through narrow layouts, scrolling, focus, and temporary-view stacking.
    - [ ] **45e. Add inspection evidence.** Test long, missing, hostile, ambiguous, and fallback metadata directly.
46. **Complete the privacy audit.**
    - [ ] **46a. Audit network and process surfaces.** Prove no networking or undeclared child process occurs outside confirmed link launch.
    - [ ] **46b. Audit filesystem reads.** Prove no directory scanning, unrelated reads, source rewriting, or sidecar creation.
    - [ ] **46c. Audit persisted data.** Enumerate every stored field and prove no undocumented path, content, telemetry, or identifier is retained.
    - [ ] **46d. Audit diagnostics and captures.** Sanitize private values and controls in errors, logs, test artifacts, and hosted output.
    - [ ] **46e. Publish privacy evidence.** Update cases, policy, known limitations, and exact audit commands and results.
47. **Complete Paper and accessibility matrices.**
    - [ ] **47a. Generate the state matrix.** Enumerate color capability, viewport, focus, search, selection, annotation, warning, image, loading, and error combinations.
    - [ ] **47b. Assert color and contrast.** Check semantic roles and required contrast for every supported Paper capability pairing.
    - [ ] **47c. Assert non-color identification.** Verify focus, matches, selection, annotations, warnings, loading, and errors remain distinguishable without color.
    - [ ] **47d. Assert responsive geometry.** Check every matrix state at wide, standard, compact, narrow, and below-minimum dimensions.
    - [ ] **47e. Record human accessibility rows.** Execute required keyboard, screen-reader, and visual procedures and retain environment-specific results.
48. **Build the benchmark framework.**
    - [ ] **48a. Freeze benchmark environments.** Register representative hardware, OS, terminal, toolchain, power assumptions, and noise controls.
    - [ ] **48b. Register benchmark fixtures.** Select reproducible small, normal, large, hostile, search, image, and state inputs with hashes.
    - [ ] **48c. Implement timing harnesses.** Add release-profile markers, warmup, samples, statistics, and regression thresholds.
    - [ ] **48d. Implement memory and retention harnesses.** Measure peak/steady allocations, queues, caches, and post-navigation retention reproducibly.
    - [ ] **48e. Map `PERF-001` through `PERF-009`.** Register commands, resources, thresholds, and evidence output formats.
49. **Meet performance and retention budgets.**
    - [ ] **49a. Measure launch and open.** Baseline startup plus plain-text, Markdown, EPUB, raster, and SVG opening and address threshold failures.
    - [ ] **49b. Measure navigation and relayout.** Baseline line/page/section movement, resize, theme, and mode changes and optimize regressions.
    - [ ] **49c. Measure persistence.** Baseline config/state load, checkpoint, annotation save, and atomic replacement under representative state sizes.
    - [ ] **49d. Measure search scaling.** Baseline query startup, forward/backward traversal, normalization, cancellation, and result retention.
    - [ ] **49e. Measure hostile cancellation and memory.** Baseline decoder/parser cancellation, worker shutdown, queues, image buffers, and cache steady state.
    - [ ] **49f. Reconcile budget exceptions.** Fix failures or document narrow measured exceptions with owner, impact, expiry, and removal condition.
50. **Run usability sessions and finish guides.**
    - [ ] **50a. Write the usability protocol.** Define participant criteria, consent/privacy, common journeys, observations, severity, and stopping rules.
    - [ ] **50b. Run opening and reading sessions.** Observe installation/startup, opening books, navigation, layout, themes, and recovery with non-author readers.
    - [ ] **50c. Run search and annotation sessions.** Observe search, selection, bookmarks, highlights, notes, management, and persistence recovery.
    - [ ] **50d. Triage and fix usability findings.** Convert reproducible defects to tests and resolve release-blocking confusion or failure.
    - [ ] **50e. Revise reader-facing guides.** Update user, troubleshooting, accessibility, privacy, and known-limit documentation from findings.
    - [ ] **50f. Revise contributor guidance.** Document architecture, fixtures, test profiles, evidence, release checks, and extension boundaries.
51. **Optionally add coverage-guided fuzzing.**
    - [ ] **51a. Decide whether optional fuzzing fits the release budget.** Record available time, compute, disk, target value, and an explicit skip if it does not.
    - [ ] **51b. Add bounded fuzz targets.** If approved, cover highest-risk parser/state/action surfaces with limits and deterministic seed corpora.
    - [ ] **51c. Run scheduled discovery campaigns.** Retain toolchain, duration, corpus, coverage, crashes, and resource use without making duration a release gate.
    - [ ] **51d. Promote every defect.** Minimize each crash or invariant failure into a deterministic regression before fixing it.

## Phase 5

52. **Finalize support and packaging policy.**
    - [ ] **52a. Freeze platform tuples.** Select promised OS versions, architectures, libc/runtime assumptions, and evidence environments.
    - [ ] **52b. Freeze terminal tuples.** Select terminals, sessions, SSH/tmux combinations, color modes, Unicode limits, and image protocols.
    - [ ] **52c. Freeze package channels.** Select archive/installer/package-manager outputs, installation scope, update expectations, and signing requirements.
    - [ ] **52d. Assign evidence and deferrals.** Name owners for each tuple and document unsupported or deferred combinations precisely.
    - [ ] **52e. Reconcile support documents.** Update policy, matrix, README, procedures, and release gates to the same promises.
53. **Configure cargo-dist native artifacts.**
    - [ ] **53a. Configure release targets.** Add only frozen OS/architecture tuples with locked toolchain and reproducible cargo-dist settings.
    - [ ] **53b. Configure archives and installers.** Define filenames, layout, executable permissions, licenses, notices, shell completions, and metadata.
    - [ ] **53c. Generate checksums and manifests.** Emit versioned artifact inventory, hashes, source revision, tool versions, and target identity.
    - [ ] **53d. Enforce protected-tag traceability.** Bind artifacts to an allowed tag and commit and reject dirty, untagged, or mismatched builds.
    - [ ] **53e. Test artifact contents locally.** Inspect every produced format for exact files, permissions, metadata, and absence of workspace leakage.
54. **Harden release automation.**
    - [ ] **54a. Pin workflow dependencies.** Replace floating action references with reviewed immutable revisions and record update ownership.
    - [ ] **54b. Minimize workflow permissions.** Give each job only required read, artifact, attestation, or release rights and isolate untrusted inputs.
    - [ ] **54c. Restrict release origins.** Permit publication only from protected tags and matching commits after required environments and gates pass.
    - [ ] **54d. Validate workflow identity.** Assert repository, ref, commit, event, actor policy, and artifact provenance before publication.
    - [ ] **54e. Add release workflow tests.** Exercise dry run, wrong tag, wrong commit, missing gate, tampered artifact, and successful staging paths.
55. **Complete dependency and notice manifests.**
    - [ ] **55a. Audit enabled dependency graph.** Review licenses, advisories, sources, duplicates, features, and every enabled image decoder.
    - [ ] **55b. Audit non-Cargo assets.** Reconcile fixtures, fonts if any, icons, captures, generated files, and documentation assets with provenance.
    - [ ] **55c. Generate third-party notices.** Produce deterministic notices matching the locked graph and packaged assets.
    - [ ] **55d. Verify artifact notice inclusion.** Ensure each package carries required project and third-party license material and correct versions.
    - [ ] **55e. Record supply-chain evidence.** Retain commands, exceptions, owners, expiry, and removal conditions.
56. **Test clean installation and first run.**
    - [ ] **56a. Write clean-account procedures.** Define prerequisites, isolation, cache state, commands, expected paths, and cleanup for every package channel.
    - [ ] **56b. Test Linux installation.** Install on clean supported Linux accounts and verify help, version, fixture opening, reading, and clean exit.
    - [ ] **56c. Test macOS installation.** Run the same checks on clean supported macOS accounts and record platform-specific prompts or limits.
    - [ ] **56d. Test Windows installation.** Run the same checks on clean supported Windows accounts and record PATH, ConPTY, and cleanup behavior.
    - [ ] **56e. Reconcile installation guides.** Fix package or documentation defects and retain final commands, artifacts, and environment evidence.
57. **Test upgrade and rollback disposition.**
    - [ ] **57a. Name predecessor and compatibility scope.** Select the prior release or explicitly record first-release non-applicability.
    - [ ] **57b. Test forward migration.** Install the new release over predecessor state/configuration and verify data, behavior, and failure recovery.
    - [ ] **57c. Test rollback behavior.** Reinstall the predecessor where supported and verify documented compatibility or refusal without silent loss.
    - [ ] **57d. Test interrupted upgrade.** Inject artifact, install, migration, and first-write failures and preserve a recoverable installation and state.
    - [ ] **57e. Publish upgrade/rollback procedure.** Record exact supported paths, backups, limitations, and future first-upgrade requirements.
58. **Run the final native terminal matrix.**
    - [ ] **58a. Freeze the final matrix revision.** Push one clean candidate with artifacts, manifests, procedures, and all local cumulative gates passing.
    - [ ] **58b. Run Linux terminal tuples.** Execute build, core, doctests, PTY, install, reading, feature, resize/session, image, restoration, and exit checks.
    - [ ] **58c. Run macOS terminal tuples.** Execute the same applicable matrix on every promised macOS terminal/session tuple.
    - [ ] **58d. Run Windows terminal tuples.** Execute the same applicable matrix on every promised Windows/ConPTY tuple.
    - [ ] **58e. Run SSH and multiplexer tuples.** Execute the promised remote and tmux/session combinations, including fallback and restoration.
    - [ ] **58f. Resolve and rerun failures.** Add deterministic regressions, fix defects, rebuild artifacts, and rerun every invalidated row at one candidate revision.
    - [ ] **58g. Publish final matrix evidence.** Link every promised tuple to exact revision, artifact, environment, result, capture, and known limitation.
59. **Publish captures and known limitations.**
    - [ ] **59a. Capture representative UI.** Record delivered wide, narrow, search, selection, annotation, help, error, and image states from release artifacts.
    - [ ] **59b. Document text boundaries.** State evidenced Unicode, grapheme, width, bidi, locale, control, and screen-reader behavior precisely.
    - [ ] **59c. Document terminal boundaries.** State supported terminals, sessions, color, resize, input, restoration, and non-TTY behavior precisely.
    - [ ] **59d. Document image boundaries.** State decoder, resource, protocol, geometry, fallback, animation, and native acceptance limits precisely.
    - [ ] **59e. Reconcile public documentation.** Ensure README, guides, accessibility, privacy, support, captures, and release notes make identical claims.
60. **Execute the release and rollback checklist.**
    - [ ] **60a. Verify cumulative gates and cases.** Require every release P0/P1 case, environment, manual row, and manifest freshness check at the candidate revision.
    - [ ] **60b. Verify artifacts and provenance.** Match versions, checksums, contents, notices, attestations, source commit, tag, and clean-install evidence.
    - [ ] **60c. Stage and verify release metadata.** Prepare notes, known limitations, captures, support matrix, upgrade/rollback instructions, and package links.
    - [ ] **60d. Exercise rollback before publication.** Run the documented release rollback procedure against staged metadata and artifacts.
    - [ ] **60e. Tag and publish.** Create the protected tag only after all checks pass, publish immutable artifacts, and verify public hashes and links.
    - [ ] **60f. Perform post-release verification.** Install from public channels, open fixtures, verify help/version, monitor failures, and retain the final evidence record.
