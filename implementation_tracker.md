# Implementation Tracker

**Last updated:** August 19, 2026 at 5:48 PM EDT

## Table of Contents

- [Status Key](#status-key)
- [Current Status](#current-status)
- [Foundation](#foundation)
- [Core Reader](#core-reader)
- [Library Management](#library-management)
- [User Experience](#user-experience)
- [Quality and Distribution](#quality-and-distribution)
- [Known Risks](#known-risks)

## Status Key

| Status | Meaning |
| --- | --- |
| Not started | Work has not begun. |
| In progress | Work is actively being implemented. |
| Blocked | A dependency or decision prevents progress. |
| Complete | Implementation and validation are finished. |

## Current Status

TermLeaf is in project setup and requirements definition. No application stack
or source implementation has been selected yet.

## Foundation

| Feature | Status | Notes |
| --- | --- | --- |
| Repository hygiene | Complete | Initial `.gitignore` and documentation structure added. |
| Product requirements | Not started | Define target readers, formats, and platforms. |
| Technical architecture | Blocked | Depends on product requirements and stack selection. |
| CLI command structure | Not started | Define command names, options, and exit behavior. |
| Configuration model | Not started | Define defaults, config location, and overrides. |

## Core Reader

| Feature | Status | Notes |
| --- | --- | --- |
| Plain-text rendering | Not started | Establish baseline reading flow. |
| Terminal-aware layout | Not started | Handle width, height, wrapping, and resize events. |
| Navigation | Not started | Page, line, chapter, beginning, and end controls. |
| Progress persistence | Not started | Restore the last location for each document. |
| Search | Not started | Forward and backward in-document search. |
| Format support | Not started | Select formats after requirements are confirmed. |

## Library Management

| Feature | Status | Notes |
| --- | --- | --- |
| Open local document | Not started | Validate paths and unsupported formats. |
| Recent documents | Not started | Store and display recent reading activity. |
| Library index | Not started | Optional catalog of local reading material. |
| Metadata extraction | Not started | Title, author, and document structure where available. |

## User Experience

| Feature | Status | Notes |
| --- | --- | --- |
| Keyboard controls | Not started | Provide discoverable and consistent bindings. |
| Help view | Not started | Include commands and active key bindings. |
| Themes | Not started | Respect terminal capabilities and accessibility. |
| Error reporting | Not started | Use concise, actionable terminal messages. |

## Quality and Distribution

| Feature | Status | Notes |
| --- | --- | --- |
| Automated tests | Not started | Unit, integration, and terminal behavior coverage. |
| Continuous integration | Not started | Run formatting, linting, tests, and build checks. |
| Packaging | Not started | Choose channels after platform targets are defined. |
| Release process | Not started | Versioning, changelog, artifacts, and checksums. |
| User documentation | In progress | Initial repository documents are available. |

## Known Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Undefined input formats | Architecture cannot be finalized. | Prioritize requirements and representative fixtures. |
| Terminal compatibility | Rendering may vary across environments. | Define supported terminals and add integration tests. |
| Large documents | Parsing or navigation may consume excess memory. | Establish performance targets and test incrementally. |
| Local state corruption | Reading progress could be lost. | Use atomic writes and versioned state formats. |
