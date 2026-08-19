# TermLeaf Project Plan

**Last updated:** August 19, 2026 at 5:48 PM EDT

## Table of Contents

- [Vision](#vision)
- [Goals](#goals)
- [Non-Goals](#non-goals)
- [Guiding Principles](#guiding-principles)
- [Delivery Plan](#delivery-plan)
- [Definition of Done](#definition-of-done)
- [Open Decisions](#open-decisions)
- [Maintenance](#maintenance)

## Vision

TermLeaf enables comfortable, distraction-free reading without leaving the
terminal. It should open documents quickly, render them clearly, preserve
reading progress, and remain predictable across supported terminal sizes.

## Goals

- Provide a fast terminal-first reading workflow.
- Make navigation and controls easy to discover and remember.
- Preserve reading position safely between sessions.
- Handle supported document formats with clear failure messages.
- Keep installation and configuration straightforward.
- Maintain dependable behavior through automated tests and release checks.

## Non-Goals

- Editing or authoring documents in the initial release.
- Digital rights management circumvention.
- Cloud accounts or cross-device synchronization in the initial release.
- Full parity with graphical e-reader typography.

## Guiding Principles

- Prefer a small, reliable core over broad format support.
- Keep local reading data private and usable offline.
- Treat keyboard accessibility and terminal compatibility as core behavior.
- Choose dependencies deliberately and keep startup overhead low.
- Record material design decisions in `commit_tracker.md`.
- Track feature status and blockers in `implementation_tracker.md`.

## Delivery Plan

### Phase 1: Requirements and Architecture

Deliverables:

- Define supported operating systems and terminal environments.
- Identify the first document format and representative test fixtures.
- Specify key workflows, commands, keyboard controls, and accessibility needs.
- Set measurable startup, navigation, and memory targets.
- Select the implementation stack and document the architecture.

Exit criteria:

- Open decisions required for the first vertical slice are resolved.
- A minimal application can be built and tested locally.

### Phase 2: Reading Vertical Slice

Deliverables:

- Open one supported local document format.
- Render readable, terminal-width-aware content.
- Navigate forward, backward, to the beginning, and to the end.
- Display basic location or progress information.
- Handle invalid paths, unreadable files, and unsupported content.

Exit criteria:

- A user can complete a reading session against representative fixtures.
- Core behavior has automated tests.

### Phase 3: Persistent Reader

Deliverables:

- Save and restore reading position safely.
- React correctly to terminal resize events.
- Add in-document search and help views.
- Define configuration precedence and storage locations.
- Validate performance with large documents.

Exit criteria:

- State survives normal exits and interrupted writes without corruption.
- Supported terminal environments pass integration checks.

### Phase 4: Library and Polish

Deliverables:

- Add recent-document history and optional library indexing.
- Present available metadata where the source format supports it.
- Refine themes, status information, and error messages.
- Complete user and contributor documentation.

Exit criteria:

- Primary workflows are documented and usability-tested.
- Accessibility and performance targets are met.

### Phase 5: Release

Deliverables:

- Automate formatting, linting, tests, and release builds.
- Package the application for supported platforms.
- Establish versioning, changelog, artifact, and checksum procedures.
- Run installation and upgrade smoke tests.

Exit criteria:

- Release artifacts are reproducible and pass all quality gates.
- Installation and first-use instructions are verified from a clean system.

## Definition of Done

A feature is complete when:

- Its expected behavior and edge cases are documented.
- The implementation follows established project conventions.
- Automated tests cover its critical behavior.
- Formatting, linting, tests, and builds pass.
- User-facing documentation and trackers reflect the change.
- Relevant design decisions are recorded with the associated commit.

## Open Decisions

| Decision | Why It Matters | Target Phase |
| --- | --- | --- |
| Implementation language and terminal UI library | Determines architecture, packaging, and test strategy. | Phase 1 |
| Initial document format | Defines parsing and navigation requirements. | Phase 1 |
| Supported operating systems and terminals | Sets compatibility and release scope. | Phase 1 |
| State and configuration locations | Affects portability, privacy, and upgrades. | Phase 1 |
| Navigation and key-binding model | Shapes the core reading experience. | Phase 1 |
| Packaging channels | Determines release automation requirements. | Phase 5 |

## Maintenance

- Update this plan when scope, sequencing, or acceptance criteria change.
- Update `implementation_tracker.md` as feature status changes.
- Update `commit_tracker.md` with every commit and material design decision.
- Refresh each document's **Last updated** timestamp when its content changes.
