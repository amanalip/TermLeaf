# TermLeaf Project Plan

**Last updated:** August 19, 2026 at 6:20 PM EDT

## Table of Contents

- [The Idea](#the-idea)
- [What Success Looks Like](#what-success-looks-like)
- [What Can Wait](#what-can-wait)
- [Rules for the Build](#rules-for-the-build)
- [Roadmap](#roadmap)
- [When a Feature Is Finished](#when-a-feature-is-finished)
- [Questions to Settle](#questions-to-settle)
- [Keeping the Plan Honest](#keeping-the-plan-honest)

## The Idea

Reading in a terminal should feel intentional, not like opening a text file and
making do. TermLeaf aims for the quiet parts of a good e-reader: crisp pages,
quick movement, a reliable bookmark, and controls that soon become muscle
memory. It should start fast, work offline, and leave the reader's library on
their own machine.

## What Success Looks Like

- A book opens quickly into a view that is comfortable for more than a few
  minutes of reading.
- Moving around feels immediate, whether the reader advances one line or jumps
  across chapters.
- Closing the program is safe. The next session starts at the same passage.
- A narrow window, a resized terminal, or an awkward file produces sensible
  behavior instead of a broken screen.
- Installation is short enough to explain clearly and dependable enough to
  trust.

## What Can Wait

The first release does not need to edit books, sync reading progress through a
cloud account, or reproduce every typographic detail of a graphical e-reader.
It will not circumvent digital rights management. These boundaries leave room
to make the basic act of reading genuinely good.

## Rules for the Build

- Do one format well before collecting half-finished parsers.
- Keep books, history, and reading positions local by default.
- Treat keyboard access and terminal compatibility as part of the reader, not
  polish for later.
- Be cautious with dependencies and protective of startup time.
- Record choices with lasting consequences in `commit_tracker.md`.
- Keep the honest state of each feature in `implementation_tracker.md`.

## Roadmap

### 1. Draw the Boundaries

First, decide who the initial release is for and where it must run. Choose the
first document format, gather a handful of representative books, sketch the
commands and keys, and set concrete speed and memory expectations. Only then
should the project choose its language and terminal UI library.

We can move on when a tiny application builds locally and the open questions
needed for the first reading loop have answers.

### 2. Make Reading Possible

Build the shortest complete journey: open one supported book, lay it out at
the current terminal width, move forward and backward, jump to either end, and
show enough progress to keep the reader oriented. Bad paths and malformed
books should produce useful messages rather than stack traces or blank screens.

This milestone is ready when someone can sit down with a representative book,
read for a while, and navigate without fighting the interface. Tests should
cover that same journey.

### 3. Make Reading Dependable

Add the details that turn a demo into a daily tool. Save positions safely,
restore them on the next launch, respond to terminal resizing, search inside a
book, and keep help close at hand. Define where configuration and local state
live. Put large books through the same path before performance problems harden
into the design.

This stage is complete when interrupted writes do not destroy progress and the
reader behaves consistently in every terminal we claim to support.

### 4. Build the Bookshelf

Once the reading loop is solid, make returning easier. Add recent books, expose
useful metadata, and consider a local library index. Refine the status line,
colors, and error messages with real use rather than decoration for its own
sake. Finish the guides a new reader and a new contributor will actually need.

We are finished here when the common paths have been tried by people other
than the author and the agreed accessibility and performance targets hold up.

### 5. Ship It

Automate the checks that protect a release, produce packages for supported
platforms, and write down the versioning and changelog routine. Build artifacts
should be reproducible, checksummed, and tested from a clean machine.

The release is ready when a new user can install TermLeaf, open a book, and
start reading by following the published instructions exactly.

## When a Feature Is Finished

A feature earns **Complete** in the tracker when its behavior is clear, its
important edges are tested, and all project checks pass. Reader-facing changes
must be reflected in the docs. Choices that could puzzle a future contributor
belong in the decision log attached to the same commit.

## Questions to Settle

| Question | Why answer it early? | When |
| --- | --- | --- |
| Which language and terminal UI library fit TermLeaf? | This shapes the architecture, packaging, and test approach. | Stage 1 |
| Which document format comes first? | Parsing, structure, and navigation all depend on it. | Stage 1 |
| Which systems and terminals do we promise to support? | A promise needs repeatable compatibility checks. | Stage 1 |
| Where should settings and reading positions live? | The answer affects privacy, portability, and upgrades. | Stage 1 |
| Which navigation style should feel native? | Key choices define the rhythm of everyday reading. | Stage 1 |
| How will people install TermLeaf? | Release automation should serve real distribution channels. | Stage 5 |

## Keeping the Plan Honest

This plan should change when evidence changes the route, not simply because a
date passed. Update `implementation_tracker.md` as work moves or stalls. Update
`commit_tracker.md` whenever a commit changes behavior or settles an important
question. Refresh the timestamp on any document whose meaning changes.
