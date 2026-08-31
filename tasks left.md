# Tasks Left

This is the ordered implementation queue for the first release. Task numbers
remain stable; completed tasks should be struck through rather than removed.
Remaining tasks are sized so one contributor can usually finish one, including
focused tests and documentation, within a five-hour work window. They are
estimates, not deadlines: split a task before starting if investigation reveals
materially larger scope.

“Implement the next 10” means the first ten incomplete numbered tasks from the
top. Tasks run in listed order unless their text explicitly permits parallel
work. Unnumbered headings group related tasks and do not count toward a batch.

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

### Implement terminal graphics capability probing
12. ~~**Freeze query policy.** Specify query bytes, response limits, timeout values, parser states, precedence, and fallback behavior for Kitty, Sixel, and iTerm2 without changing case status.~~
13. ~~**Build the bounded probe transport.** Send queries and collect partial responses under byte and time limits while preserving unrelated input for the event loop.~~
14. ~~**Parse positive capability evidence.** Accept only protocol-specific complete responses and reject malformed, partial, delayed, absent, and spoofed values with typed outcomes.~~
15. ~~**Integrate one-shot backend selection.** Apply locked override and protocol precedence once, emit no competing protocol, and retain cell/caption fallback.~~
16. ~~**Complete deterministic `IMG-017` evidence.** Add response tables and timeout tests, register locations, and record exact local checks without claiming hosted PTY behavior.~~

### Prove image lifecycle behavior over PTY
17. ~~**Extend the PTY harness for image protocols.** Capture probe and graphics bytes with deterministic deadlines, dimensions, child cleanup, and protocol-aware assertions.~~
18. ~~**Prove display, replacement, and scrolling.** Exercise native placement, unchanged frames, replacement IDs, viewport entry/exit, and stale-image deletion.~~
19. ~~**Prove fallback and cancellation.** Exercise absent/malformed capability responses, missing geometry, decode or encode failure, navigation cancellation, and current-generation recovery.~~
20. ~~**Prove shutdown and restoration.** Terminate during queued and active image work and assert bounded joins, cleanup attempts, terminal restoration, and no surviving child or worker.~~
21. ~~**Register PTY lifecycle evidence.** Map the journeys to `TERM-009`, `IMG-017`, and supporting `IMG-018` locations while leaving manual acceptance Planned.~~

### Write and execute the `IMG-018` native procedure
22. ~~**Write the protocol-neutral procedure.** Define prerequisites, fixture, dimensions, display, resize, scroll, replacement, failure, exit, capture, and cleanup observations.~~
23. **Execute and record Kitty acceptance.** Run the procedure on one finalized claimed Kitty terminal and retain terminal/version evidence and results.
24. **Execute and record Sixel acceptance.** Run the procedure on one finalized claimed Sixel terminal and retain geometry, terminal/version evidence, and results.
25. ~~**Execute and record iTerm2 acceptance.** Removed from the Linux-only release scope; no iTerm2 compatibility claim.~~
26. **Reconcile native claims.** Mark only evidenced Linux protocol tuples accepted, document exclusions precisely, and update `IMG-018` status and links.

### Run the complete Phase 2 hosted matrix
27. **Prepare one gate revision.** Ensure a clean pushed revision, fresh manifests, deterministic fixtures, and exact profile commands before triggering hosted jobs.
28. **Run Linux profiles.** Retain registry, core, render, security, dependency, MSRV, and native PTY outcomes with run URLs and revision.
29. ~~**Run macOS profiles.** Removed from the Linux-only test and support scope.~~
30. ~~**Run Windows profiles.** Removed from the Linux-only test and support scope.~~
31. **Resolve hosted-only failures.** Convert defects to deterministic regressions, fix them, and rerun the complete affected matrix at one new pushed revision.
32. **Record the final hosted matrix.** Map every required environment row and profile to the passing revision without combining evidence from incompatible revisions.

### Close the Phase 2 gate
33. **Audit case status.** Compare every Phase 2 case with executable, hosted, and manual evidence; preserve Planned or Blocked status where evidence is incomplete.
34. **Regenerate and verify gate artifacts.** Refresh case, profile, fixture, and phase-gate manifests and run every local gate command.
35. **Document limits and skips.** Record native-terminal boundaries, unsupported combinations, optional fuzz non-runs, and residual risks.
36. **Update release-facing documents.** Reconcile README, implementation and commit trackers, test report, and manual procedures against the audited status.
37. **Commit and push Phase 2 closure.** Review the complete diff, create logical commits, push the gate revision, and verify the branch and hosted links are synchronized.

## Phase 3

### Resolve non-TTY behavior
38. **Inventory non-TTY entry paths.** Record current stdin/stdout/stderr and terminal-initialization behavior for files, pipes, redirection, and unsupported combinations.
39. **Decide `DEC-TEST-002`.** Freeze exact piped-input and redirected-output behavior, exit codes, diagnostics, limits, and accessibility implications.
40. **Update `CLI-009` and `A11Y-005`.** Reconcile catalog, procedures, manifests, and implementation prerequisites with the decision.

### Freeze text and persisted-state limits
41. **Inventory bounded text surfaces.** List paths, paste, queries, notes, URLs, names, metadata, diagnostics, and configuration strings with current implicit limits.
42. **Freeze text-input limits.** Set byte, scalar, grapheme, line, and control-character policies for interactive and imported text.
43. **Freeze collection and nesting limits.** Set capacities for recents, history, annotations, configuration, versions, nesting, entries, and total persisted bytes.
44. **Record `DEC-TEST-012`.** Update cases and design documents with inclusive boundaries, rejection order, and recovery behavior.

### Resolve configuration error behavior
45. **Define syntax and type failures.** Freeze outcomes for malformed TOML, invalid values, unknown keys, and unsupported versions.
46. **Define I/O and precedence failures.** Freeze outcomes for missing, unreadable, unsafe, and platform-path failures plus CLI/config/default precedence.
47. **Record `DEC-TEST-005`.** Specify typed diagnostics, startup fallback, rewrite prohibition, and exact case updates.

### Resolve search semantics
48. **Freeze matching semantics.** Decide literal smart-case behavior, Unicode normalization, grapheme boundaries, block crossing, and control characters.
49. **Freeze navigation semantics.** Decide forward/backward start points, current-match behavior, wrapping, result ordering, and resize effects.
50. **Freeze query/history semantics.** Decide query limits, empty input, deduplication, capacity, persistence, removal, and clearing.
51. **Record `DEC-TEST-004`.** Update cases and design documents with examples and exact expected ranges.

### Define document identity and relocation
52. **Evaluate identity evidence.** Compare canonical path, metadata, content fingerprints, and privacy/storage costs against required move and edit behavior.
53. **Freeze identity and stale-book outcomes.** Decide moved, missing, replaced, edited, duplicate, and inaccessible source behavior.
54. **Freeze anchor relocation evidence.** Define exact-match and contextual evidence, ambiguity handling, and unresolved preservation.
55. **Record `DEC-TEST-006`.** Update schemas, cases, privacy claims, and migration prerequisites.

### Define state-writer concurrency
56. **Choose writer linearization.** Compare locking and optimistic replacement and freeze ownership, timeout, and same-process behavior.
57. **Define recovery and races.** Specify stale-owner recovery, migration races, concurrent updates, and last-writer outcomes.
58. **Record `DEC-TEST-007`.** Update state cases, fault matrix, platform assumptions, and implementation contract.

### Freeze remaining Phase 3 interactions
59. **Freeze help and temporary-view navigation.** Decide invocation scope, return stacking, nested temporary views, focus restoration, and cancellation.
60. **Freeze selection interactions.** Decide keys, anchor/caret behavior, reversal, cross-block ranges, resize, and cancellation.
61. **Freeze note editing interactions.** Decide entry, multiline keys, save/cancel, paste, limits, and failure recovery.
62. **Freeze highlight identifiers.** Define versioned colors, non-color labels, migration, contrast, and screen-reader names.

### Complete the configuration schema
63. **Define typed configuration models.** Implement defaults, deserialization, validation, version handling, and exact field limits.
64. **Resolve platform-native paths.** Implement explicit and default path discovery without scanning or creating unrelated files.
65. **Implement precedence and diagnostics.** Apply CLI over config over defaults and surface frozen warnings or typed errors without rewriting TOML.
66. **Complete `CFG-001` through `CFG-004`.** Add boundary, malformed, unknown-key, precedence, and no-rewrite evidence and register it.

### Implement versioned state models
67. **Define current state envelope.** Implement version, document identity, settings, and bounded top-level collections.
68. **Implement position and recent models.** Add validated logical anchors, metadata snapshots, ordering fields, and round trips.
69. **Implement search and annotation models.** Add bounded history, bookmarks, highlights, notes, identifiers, and unresolved states.
70. **Implement migration dispatch.** Decode current and supported old versions deterministically and reject future versions without data loss.
71. **Add schema round-trip evidence.** Cover empty, maximum, old, current, future, malformed, and cross-field-invalid states.

### Implement atomic durable state writes
72. **Implement same-directory temporary writes.** Create private temporary state beside the destination and serialize under the frozen size limit.
73. **Implement sync and replacement.** Apply the chosen file and directory sync order and platform replacement policy.
74. **Implement cleanup and permissions.** Remove abandoned temporaries where safe and verify private permissions on creation and replacement.
75. **Add write fault injection.** Fail each open, write, flush, sync, rename, and cleanup step while preserving the prior valid state.

### Secure concurrent state storage
76. **Reject unsafe state paths.** Deny symlinks, non-regular files, ownership or permission violations, and path swaps at each opening boundary.
77. **Implement writer coordination.** Apply the chosen lock or linearization policy with bounded wait and stale-owner recovery.
78. **Enforce aggregate state limits.** Reject excessive bytes, nesting, entries, text, and collections before unbounded allocation.
79. **Complete `STATE-009` and `STATE-011` through `STATE-015`.** Add deterministic race, swap, limit, fault, and recovery evidence.

### Restore and checkpoint reading positions
80. **Persist logical reading anchors.** Save bounded per-book section, block, source offset, mode, and contextual relocation evidence.
81. **Restore exact anchors.** Reopen unchanged books at the same logical passage across viewport, theme, and mode differences.
82. **Recover stale anchors.** Apply the frozen relocation policy for edited, moved, missing, or ambiguous books and retain unresolved state safely.
83. **Implement checkpoint triggers.** Save on bounded cadence, navigation milestones, normal exit, and supported interruption without blocking terminal cleanup.
84. **Add position lifecycle journeys.** Cover resize, mode, theme, crash-safe prior state, interruption, and stale recovery.

### Implement recent books
85. **Implement bounded MRU updates.** Insert, deduplicate, reorder, cap, and persist recents using frozen document identity.
86. **Implement metadata and stale states.** Show trustworthy title/path fallback, current position, missing or inaccessible markers, and no directory scanning.
87. **Implement recent-book actions.** Reopen, remove, clear with confirmation, and preserve selection and return behavior.
88. **Build responsive recent views.** Render wide through narrow states with long safe paths and non-color focus cues.
89. **Complete recent-book evidence.** Add ordering, capacity, stale, persistence, UI, and privacy tests.

### Implement Open Path
90. **Build isolated text focus.** Route bounded typing, deletion, movement, paste, submit, and cancel without triggering reader bindings.
91. **Validate submitted paths.** Apply frozen text/path limits and supported-source checks without scanning or rewriting input.
92. **Implement success and recovery flow.** Open valid books, preserve editable invalid input, display typed diagnostics, and restore the invoking view exactly.
93. **Build responsive Open Path UI.** Render focus, help, validation, long input, paste, and narrow states accessibly.
94. **Complete Open Path evidence.** Add keyboard, paste, limit, no-scan, error recovery, and return-stack tests.

### Implement the search engine
95. **Build normalized searchable text mapping.** Map normalized comparison text back to exact source and grapheme-safe logical ranges.
96. **Implement literal smart-case matching.** Cover forward and backward matching, block policy, controls, and exact boundary semantics.
97. **Implement result navigation.** Apply start-point, current-match, wrap, ordering, and end-state rules.
98. **Add generation cancellation and limits.** Bound query, work, results, memory, stale completion, and navigation-away behavior.
99. **Complete core `SEARCH` evidence.** Add normalization, direction, boundaries, wrapping, cancellation, and fixed-seed properties.

### Implement search UI and history
100. **Build isolated search entry.** Support bounded edit and paste, submit, cancel, direction, and invoking-view restoration.
101. **Render search results and feedback.** Show current/total, wrap, no-result, invalid, loading, and non-color match cues.
102. **Implement local search history.** Deduplicate, cap, navigate, persist, remove, and confirm clearing under frozen semantics.
103. **Preserve search through layout changes.** Keep logical matches and current result stable across resize, theme, and mode changes.
104. **Complete search UI evidence.** Add entry isolation, feedback, history, responsive, and accessibility journeys.

### Implement text selection
105. **Define logical selection state.** Store anchor and caret as validated grapheme-safe document positions independent of visual rows.
106. **Implement selection movement.** Start, extend, contract, reverse, cross allowed block boundaries, and cancel using frozen keys.
107. **Render accessible selection.** Add theme and non-color cues without altering source text or terminal cell width.
108. **Preserve selection through relayout.** Keep exact logical ranges across wrapping, resize, theme, and mode changes.
109. **Expose annotation attachment ranges.** Convert valid selections into bookmark/highlight/note inputs and reject unsupported ranges.

### Implement bookmarks
110. **Implement bookmark creation and validation.** Create point annotations with bounded optional names and stable document identity.
111. **Implement bookmark list and jump.** Filter current book, order stably, preview context, jump, and restore return state.
112. **Implement rename and delete.** Preserve editable failures, confirm deletion, and handle unresolved bookmarks.
113. **Persist and migrate bookmarks.** Round-trip current records and supported old forms under limits.
114. **Complete bookmark evidence.** Cover intermediate UI, duplicate names, limits, relocation, persistence, and failure recovery.

### Implement highlights
115. **Implement validated highlight ranges.** Create only supported nonempty logical ranges and reject cross-boundary invalid state.
116. **Implement versioned highlight styles.** Store stable identifiers and map them to accessible color and non-color cues.
117. **Render relayout-stable highlights.** Project logical ranges across wrapping, resize, theme, and mode without width drift.
118. **Implement highlight editing and deletion.** Change style, preserve range, confirm deletion, and recover save failures.
119. **Complete highlight evidence.** Cover overlap policy, persistence, migration, accessibility, relocation, and invalid ranges.

### Implement notes
120. **Implement point and range note models.** Validate attachment positions, bounded text, identity, timestamps or ordering, and unresolved state.
121. **Build isolated note editing.** Support multiline typing and paste, movement, save, cancel, and frozen key behavior.
122. **Render terminal-safe note content.** Preserve multiline round trips while escaping controls and handling narrow layouts.
123. **Implement save-failure recovery.** Keep unsaved text available, report pending/failed state, and retry or cancel without corrupting prior data.
124. **Complete note evidence.** Cover limits, point/range attachments, paste, controls, persistence, relocation, and write faults.

### Implement annotation management
125. **Build current-book annotation projection.** Combine bookmarks, highlights, and notes with stable ordering and unresolved markers.
126. **Build responsive annotation list.** Show type, name or preview, context, focus, empty state, and narrow layouts.
127. **Implement annotation navigation.** Jump to resolved items, show unresolved details, and restore the invoking view.
128. **Implement edit and delete dispatch.** Open the correct editor, confirm destructive actions, and preserve selection after changes.
129. **Complete management evidence.** Cover filtering, ordering, previews, jump, edit, delete, unresolved, and responsive behavior.

### Complete help and feedback views
130. **Implement contextual help sections.** Generate bindings from the registry and show only actions valid for the invoking view and focus.
131. **Implement status and theme reference.** Explain status fields, modes, themes, color capability, and non-color identifiers responsively.
132. **Implement recoverable error views.** Show sanitized reason, recovery actions, retained input, and exact return behavior.
133. **Implement persistence feedback.** Surface save-pending, saved, retryable failure, and unresolved-state messages with bounded lifetimes.
134. **Complete help and feedback evidence.** Cover every view, focus, narrow class, registry drift, failure action, and return stack.

### Harden state, configuration, and actions deterministically
135. **Build malformed state/configuration tables.** Cover syntax, types, versions, nesting, lengths, counts, truncation, and hostile controls.
136. **Add exact boundary tests.** Exercise every frozen text, collection, allocation, file, and total-state limit at below, equal, and one over.
137. **Add fixed-seed model properties.** Require arbitrary bounded state/configuration inputs to produce a valid model or typed bounded error.
138. **Add bounded mutation suites.** Mutate representative current and old persisted documents and preserve prior valid state on failure.
139. **Add action-sequence properties.** Exercise focus, temporary views, search, selection, annotation, save, resize, and cancellation invariants.
140. **Register deterministic replacements.** Map evidence replacing required `FUZZ-008` through `FUZZ-010` intent without claiming fuzz duration.

### Pass the Phase 3 native and accessibility gate
141. **Complete missing procedures.** Write exact keyboard-only, text-entry isolation, non-color, redraw, locale, and screen-reader steps and prerequisites.
142. **Execute local native/accessibility journeys.** Record `KEY-005` and supported deterministic procedure halves with captures and environment identity.
143. **Run the hosted Phase 3 matrix.** Execute required Linux MSRV, dependency, render, security, and PTY profiles at one revision.
144. **Record deferred human rows.** Execute available GUI/screen-reader rows and precisely mark unavailable environments without promotion.
145. **Audit and close the Phase 3 gate.** Reconcile cases, manifests, reports, procedures, limits, and hosted links at the passing revision.

## Phase 4

### Freeze the external-link policy
146. **Classify allowed destinations.** Decide supported schemes, internal versus external handling, and explicitly rejected syntax.
147. **Freeze URL normalization and limits.** Define parsing, length, Unicode/percent encoding, suspicious forms, fragments, and display escaping.
148. **Freeze launcher contract.** Define one-argument invocation, environment use, timeout, child status, and platform failure behavior.
149. **Record `DEC-TEST-008`.** Update cases, privacy/security claims, and implementation examples.

### Implement external-link focus and confirmation
150. **Project links into logical focus order.** Traverse links across wrapping and blocks without changing reading anchors.
151. **Implement internal and unsupported outcomes.** Navigate validated internal targets and show inert typed feedback for unsupported destinations.
152. **Build confirmation view.** Show the complete scrollable escaped destination, risk context, confirm, cancel, and invoking-view return.
153. **Guarantee single activation.** Debounce confirmation and prevent repeated launch on redraw, key repeat, or delayed child outcome.
154. **Complete link interaction evidence.** Cover focus, wrapping, internal navigation, long destinations, unsupported forms, and cancellation.

### Implement safe platform launchers
155. **Implement Unix launcher selection.** Invoke one validated URL as one non-shell argument and type missing/spawn/timeout/status failures.
156. ~~**Implement macOS launcher selection.** Removed from the Linux-only release scope.~~
157. ~~**Implement Windows launcher selection.** Removed from the Linux-only release scope.~~
158. **Add fake-launcher integration tests.** Capture argv and exercise success, missing binary, spawn error, timeout, and child failure deterministically.
159. **Register platform evidence.** Map local tests and hosted rows without claiming platforms not executed.

### Refine annotation relocation
160. **Build fixed edit fixtures.** Cover insertion, deletion, replacement, movement, duplicate context, normalization, and whole-book replacement.
161. **Relocate bookmark points.** Apply only frozen identity/context evidence and preserve ambiguous or missing points unresolved.
162. **Relocate highlight ranges.** Preserve exact ranges where supported and reject partial or ambiguous relocation.
163. **Relocate point and range notes.** Apply the corresponding point/range policy without changing note content.
164. **Complete relocation evidence.** Register deterministic outcomes for every edit fixture and annotation type.

### Refine metadata and critical-value inspection
165. **Complete book details.** Show trustworthy title, author, format, source, identity, and recognizable fallback values.
166. **Complete recent details.** Show current position, last-opened ordering, stale state, and safe available actions.
167. **Build critical-value presentation.** Wrap full escaped paths, URLs, diagnostics, and identifiers without silent truncation.
168. **Complete responsive and return behavior.** Cover wide through narrow layouts, scrolling, focus, and temporary-view stacking.
169. **Add inspection evidence.** Test long, missing, hostile, ambiguous, and fallback metadata directly.

### Complete the privacy audit
170. **Audit network and process surfaces.** Prove no networking or undeclared child process occurs outside confirmed link launch.
171. **Audit filesystem reads.** Prove no directory scanning, unrelated reads, source rewriting, or sidecar creation.
172. **Audit persisted data.** Enumerate every stored field and prove no undocumented path, content, telemetry, or identifier is retained.
173. **Audit diagnostics and captures.** Sanitize private values and controls in errors, logs, test artifacts, and hosted output.
174. **Publish privacy evidence.** Update cases, policy, known limitations, and exact audit commands and results.

### Complete Paper and accessibility matrices
175. **Generate the state matrix.** Enumerate color capability, viewport, focus, search, selection, annotation, warning, image, loading, and error combinations.
176. **Assert color and contrast.** Check semantic roles and required contrast for every supported Paper capability pairing.
177. **Assert non-color identification.** Verify focus, matches, selection, annotations, warnings, loading, and errors remain distinguishable without color.
178. **Assert responsive geometry.** Check every matrix state at wide, standard, compact, narrow, and below-minimum dimensions.
179. **Record human accessibility rows.** Execute required keyboard, screen-reader, and visual procedures and retain environment-specific results.

### Build the benchmark framework
180. **Freeze benchmark environments.** Register representative hardware, OS, terminal, toolchain, power assumptions, and noise controls.
181. **Register benchmark fixtures.** Select reproducible small, normal, large, hostile, search, image, and state inputs with hashes.
182. **Implement timing harnesses.** Add release-profile markers, warmup, samples, statistics, and regression thresholds.
183. **Implement memory and retention harnesses.** Measure peak/steady allocations, queues, caches, and post-navigation retention reproducibly.
184. **Map `PERF-001` through `PERF-009`.** Register commands, resources, thresholds, and evidence output formats.

### Meet performance and retention budgets
185. **Measure launch and open.** Baseline startup plus plain-text, Markdown, EPUB, raster, and SVG opening and address threshold failures.
186. **Measure navigation and relayout.** Baseline line/page/section movement, resize, theme, and mode changes and optimize regressions.
187. **Measure persistence.** Baseline config/state load, checkpoint, annotation save, and atomic replacement under representative state sizes.
188. **Measure search scaling.** Baseline query startup, forward/backward traversal, normalization, cancellation, and result retention.
189. **Measure hostile cancellation and memory.** Baseline decoder/parser cancellation, worker shutdown, queues, image buffers, and cache steady state.
190. **Reconcile budget exceptions.** Fix failures or document narrow measured exceptions with owner, impact, expiry, and removal condition.

### Run usability sessions and finish guides
191. **Write the usability protocol.** Define participant criteria, consent/privacy, common journeys, observations, severity, and stopping rules.
192. **Run opening and reading sessions.** Observe installation/startup, opening books, navigation, layout, themes, and recovery with non-author readers.
193. **Run search and annotation sessions.** Observe search, selection, bookmarks, highlights, notes, management, and persistence recovery.
194. **Triage and fix usability findings.** Convert reproducible defects to tests and resolve release-blocking confusion or failure.
195. **Revise reader-facing guides.** Update user, troubleshooting, accessibility, privacy, and known-limit documentation from findings.
196. **Revise contributor guidance.** Document architecture, fixtures, test profiles, evidence, release checks, and extension boundaries.

### Optionally add coverage-guided fuzzing
197. **Decide whether optional fuzzing fits the release budget.** Record available time, compute, disk, target value, and an explicit skip if it does not.
198. **Add bounded fuzz targets.** If approved, cover highest-risk parser/state/action surfaces with limits and deterministic seed corpora.
199. **Run scheduled discovery campaigns.** Retain toolchain, duration, corpus, coverage, crashes, and resource use without making duration a release gate.
200. **Promote every defect.** Minimize each crash or invariant failure into a deterministic regression before fixing it.

## Phase 5

### Finalize support and packaging policy
201. **Freeze platform tuples.** Select promised Linux versions, architectures, libc/runtime assumptions, and evidence environments.
202. **Freeze terminal tuples.** Select terminals, sessions, SSH/tmux combinations, color modes, Unicode limits, and image protocols.
203. **Freeze package channels.** Select archive/installer/package-manager outputs, installation scope, update expectations, and signing requirements.
204. **Assign evidence and deferrals.** Name owners for each tuple and document unsupported or deferred combinations precisely.
205. **Reconcile support documents.** Update policy, matrix, README, procedures, and release gates to the same promises.

### Configure cargo-dist native artifacts
206. **Configure release targets.** Add only frozen OS/architecture tuples with locked toolchain and reproducible cargo-dist settings.
207. **Configure archives and installers.** Define filenames, layout, executable permissions, licenses, notices, shell completions, and metadata.
208. **Generate checksums and manifests.** Emit versioned artifact inventory, hashes, source revision, tool versions, and target identity.
209. **Enforce protected-tag traceability.** Bind artifacts to an allowed tag and commit and reject dirty, untagged, or mismatched builds.
210. **Test artifact contents locally.** Inspect every produced format for exact files, permissions, metadata, and absence of workspace leakage.

### Harden release automation
211. **Pin workflow dependencies.** Replace floating action references with reviewed immutable revisions and record update ownership.
212. **Minimize workflow permissions.** Give each job only required read, artifact, attestation, or release rights and isolate untrusted inputs.
213. **Restrict release origins.** Permit publication only from protected tags and matching commits after required environments and gates pass.
214. **Validate workflow identity.** Assert repository, ref, commit, event, actor policy, and artifact provenance before publication.
215. **Add release workflow tests.** Exercise dry run, wrong tag, wrong commit, missing gate, tampered artifact, and successful staging paths.

### Complete dependency and notice manifests
216. **Audit enabled dependency graph.** Review licenses, advisories, sources, duplicates, features, and every enabled image decoder.
217. **Audit non-Cargo assets.** Reconcile fixtures, fonts if any, icons, captures, generated files, and documentation assets with provenance.
218. **Generate third-party notices.** Produce deterministic notices matching the locked graph and packaged assets.
219. **Verify artifact notice inclusion.** Ensure each package carries required project and third-party license material and correct versions.
220. **Record supply-chain evidence.** Retain commands, exceptions, owners, expiry, and removal conditions.

### Test clean installation and first run
221. **Write clean-account procedures.** Define prerequisites, isolation, cache state, commands, expected paths, and cleanup for every package channel.
222. **Test Linux installation.** Install on clean supported Linux accounts and verify help, version, fixture opening, reading, and clean exit.
223. ~~**Test macOS installation.** Removed from the Linux-only test and support scope.~~
224. ~~**Test Windows installation.** Removed from the Linux-only test and support scope.~~
225. **Reconcile installation guides.** Fix package or documentation defects and retain final commands, artifacts, and environment evidence.

### Test upgrade and rollback disposition
226. **Name predecessor and compatibility scope.** Select the prior release or explicitly record first-release non-applicability.
227. **Test forward migration.** Install the new release over predecessor state/configuration and verify data, behavior, and failure recovery.
228. **Test rollback behavior.** Reinstall the predecessor where supported and verify documented compatibility or refusal without silent loss.
229. **Test interrupted upgrade.** Inject artifact, install, migration, and first-write failures and preserve a recoverable installation and state.
230. **Publish upgrade/rollback procedure.** Record exact supported paths, backups, limitations, and future first-upgrade requirements.

### Run the final native terminal matrix
231. **Freeze the final matrix revision.** Push one clean candidate with artifacts, manifests, procedures, and all local cumulative gates passing.
232. **Run Linux terminal tuples.** Execute build, core, doctests, PTY, install, reading, feature, resize/session, image, restoration, and exit checks.
233. ~~**Run macOS terminal tuples.** Removed from the Linux-only test and support scope.~~
234. ~~**Run Windows terminal tuples.** Removed from the Linux-only test and support scope.~~
235. **Run SSH and multiplexer tuples.** Execute the promised remote and tmux/session combinations, including fallback and restoration.
236. **Resolve and rerun failures.** Add deterministic regressions, fix defects, rebuild artifacts, and rerun every invalidated row at one candidate revision.
237. **Publish final matrix evidence.** Link every promised tuple to exact revision, artifact, environment, result, capture, and known limitation.

### Publish captures and known limitations
238. **Capture representative UI.** Record delivered wide, narrow, search, selection, annotation, help, error, and image states from release artifacts.
239. **Document text boundaries.** State evidenced Unicode, grapheme, width, bidi, locale, control, and screen-reader behavior precisely.
240. **Document terminal boundaries.** State supported terminals, sessions, color, resize, input, restoration, and non-TTY behavior precisely.
241. **Document image boundaries.** State decoder, resource, protocol, geometry, fallback, animation, and native acceptance limits precisely.
242. **Reconcile public documentation.** Ensure README, guides, accessibility, privacy, support, captures, and release notes make identical claims.

### Execute the release and rollback checklist
243. **Verify cumulative gates and cases.** Require every release P0/P1 case, environment, manual row, and manifest freshness check at the candidate revision.
244. **Verify artifacts and provenance.** Match versions, checksums, contents, notices, attestations, source commit, tag, and clean-install evidence.
245. **Stage and verify release metadata.** Prepare notes, known limitations, captures, support matrix, upgrade/rollback instructions, and package links.
246. **Exercise rollback before publication.** Run the documented release rollback procedure against staged metadata and artifacts.
247. **Tag and publish.** Create the protected tag only after all checks pass, publish immutable artifacts, and verify public hashes and links.
248. **Perform post-release verification.** Install from public channels, open fixtures, verify help/version, monitor failures, and retain the final evidence record.
