# TermLeaf UI Mockups

**Last updated:** August 20, 2026 at 12:05 AM EDT

## Table of Contents

- [Purpose](#purpose)
- [Authority and Scope](#authority-and-scope)
- [Design Principles](#design-principles)
- [ASCII Conventions](#ascii-conventions)
- [Responsive Model](#responsive-model)
- [Application States](#application-states)
- [Shared Screen Anatomy](#shared-screen-anatomy)
- [Recent Books](#recent-books)
- [Open Path](#open-path)
- [Paged Reader](#paged-reader)
- [Continuous Reader](#continuous-reader)
- [Compact Reader](#compact-reader)
- [Link Focus and Text Selection](#link-focus-and-text-selection)
- [Search](#search)
- [Table of Contents](#table-of-contents)
- [Bookmarks and Annotations](#bookmarks-and-annotations)
- [Bookmark Dialog](#bookmark-dialog)
- [Highlight Dialog](#highlight-dialog)
- [Note Editor](#note-editor)
- [Theme Selection](#theme-selection)
- [External Link Confirmation](#external-link-confirmation)
- [Help](#help)
- [Image States](#image-states)
- [Loading and Background Work](#loading-and-background-work)
- [Errors and Recovery](#errors-and-recovery)
- [Destructive Confirmations](#destructive-confirmations)
- [Terminal Too Small](#terminal-too-small)
- [Status Line Rules](#status-line-rules)
- [Theme Roles](#theme-roles)
- [Focus and Input](#focus-and-input)
- [Overlay Rules](#overlay-rules)
- [Accessibility](#accessibility)
- [Implementation Guidance](#implementation-guidance)
- [Render Test Mapping](#render-test-mapping)
- [Open UI Decisions](#open-ui-decisions)
- [Phase Ownership](#phase-ownership)
- [Mockup Completion Rules](#mockup-completion-rules)

## Purpose

This document gives contributors and implementation agents a concrete visual
model for TermLeaf. The mockups show information hierarchy, layout behavior,
focus, overlays, empty and error states, and responsive degradation. They are
not pixel-perfect terminal screenshots and do not override reader behavior,
security limits, or accessibility requirements in `project_plan.md`.

The examples deliberately use ASCII only. Actual rendering may use Ratatui
borders, styles, and terminal color when the selected theme and capability allow
them. Text, ordering, focus, and logical behavior must remain understandable
without those visual enhancements.

## Authority and Scope

Resolve UI questions in this order:

1. Reader-visible behavior and safety policy in `project_plan.md`.
2. Accessibility and engineering rules in `code_quality.md`.
3. Stable verification requirements in `testcases.md`.
4. Layout and interaction intent in this document.
5. Current implementation state in `implementation_tracker.md`.

Mockups are normative for hierarchy and required information. Exact punctuation,
column widths, and provisional labels may change when render tests or native
terminal evidence show a clearer result. Any change that removes information,
changes an interaction, or moves work between phases must update the governing
plan and test IDs rather than changing only a snapshot.

Included here:

- Recent-books home screen.
- Paged and continuous reading.
- Search, table of contents, bookmarks, highlights, and notes.
- Help, status, links, images, loading, errors, and narrow terminals.
- Dark, light, high-contrast, monochrome, and Paper role usage.

Not represented as first-release screens:

- Automatic library scanning or a permanent library index.
- Accounts, cloud synchronization, downloads, plugins, or editing books.
- PDF or fixed-layout EPUB rendering.
- A mouse-only interaction.

## Design Principles

- **The passage is primary.** Reader text receives the largest stable region.
- **One obvious focus.** At most one control, row, field, or logical range is the
  active keyboard target.
- **Modes are explicit.** Search entry, note editing, confirmations, and lists
  visibly identify themselves so letter keys do not surprise the reader.
- **Logical position survives presentation.** Resize, theme, help, overlays, and
  reading-mode switches must not convert visual rows into durable positions.
- **Information collapses before content.** Secondary status fields and outer
  decoration disappear before the readable passage becomes unusable.
- **No color-only meaning.** Focus, search, selection, annotations, warning, and
  unavailable state also use text, underline, reverse, labels, or symbols.
- **Errors preserve context.** Recoverable failures appear near the current
  task; fatal startup failures restore the terminal before plain diagnostics.
- **Background work stays quiet.** Parsing and image work show bounded progress
  without animation that causes constant redraw.
- **The source remains local.** No screen implies cloud state, automatic
  scanning, remote metadata, or source modification.

## ASCII Conventions

| Mark | Meaning |
| --- | --- |
| `+---+` | Visible panel or page boundary when space permits |
| `>` | Focused row or selected list item |
| `[label]` | Action label, state label, or compact button |
| `( )` and `(*)` | Single-choice option and selected option |
| `[ ]` and `[x]` | Toggle option and enabled option |
| `...` | Content continues or a value is intentionally shortened |
| `\|` | Panel edge or vertical separation, not a source-book character |
| `^` and `v` | More content exists above or below |
| `!` | Warning that also includes readable warning text |

Key labels in mockups are illustrative unless the binding is already locked in
the product contract. The final conflict-free key map remains a Stage 1 decision
and must come from one action registry shared by input handling and help.

## Responsive Model

The examples use four descriptive classes, not hard-coded breakpoints. Stage 1
chooses thresholds from measured minimum content width and status requirements.

| Class | Example canvas | Behavior |
| --- | --- | --- |
| Wide | `120x40` | Centered page, generous margins, optional side panel |
| Standard | `80x24` | Main production layout, overlays centered |
| Compact | Around `60x18` | Smaller padding, shorter labels, one main panel |
| Narrow | Around `40x10` | Full-canvas content, one-line status, full-screen temporary views |
| Below minimum | Smaller than usable content rules | Safe size message only; no clipped controls |

Responsive order:

1. Remove unused outer canvas.
2. Reduce page or panel padding.
3. Collapse low-priority status fields.
4. Replace side-by-side panels with one full-screen panel.
5. Replace modal overlays with full-screen temporary views.
6. Remove decorative boundaries.
7. Show the terminal-too-small state if essential content and controls still do
   not fit.

The same logical anchor and focused item survive transitions whenever that item
still exists.

## Application States

```text
Startup
  |
  +-- terminal setup fails ----------> Restored terminal + plain error
  |
  +-- no path ----------------------> Recent books
  |                                     |
  |                                     +--> Open path
  |                                     |      |
  |                                     |      +--> validation error in view
  |                                     |      +--> Reader
  |                                     |
  |                                     +--> Reader
  |
  +-- supported command path --------> Reader
  |
  +-- command path cannot open ------> Restored terminal + plain error or
                                        recoverable home error by final policy

Reader
  |
  +--> Search entry/results
  +--> Link focus
  +--> Text selection
  +--> Table of contents
  +--> Bookmarks and annotations
  +--> Bookmark dialog
  +--> Highlight dialog
  +--> Note editor
  +--> External link confirmation
  +--> Help
  +--> Recoverable error message
  +--> Recent books or exit
```

Temporary views remember the originating application mode, logical passage,
focus, and scroll offset needed to return predictably.

## Shared Screen Anatomy

Standard full-screen states use three conceptual bands:

```text
+------------------------------------------------------------------------------+
| Header or context line                                                       |
+------------------------------------------------------------------------------+
|                                                                              |
| Main content or temporary view                                               |
|                                                                              |
+------------------------------------------------------------------------------+
| Status, mode, message, and compact action hints                              |
+------------------------------------------------------------------------------+
```

The header may disappear in the reader when the status line already carries its
essential context. The status band is normally one row. A temporary message may
replace lower-priority status fields but must not change content height during
its lifetime.

## Recent Books

### Standard Home

```text
+------------------------------------------------------------------------------+
| TermLeaf                                           Local books, no scanning   |
| Turn pages without leaving the terminal.                                  [?]|
+------------------------------------------------------------------------------+
| Recent books                                                                 |
|                                                                              |
| > Pride and Prejudice                         Jane Austen                     |
|   /home/reader/books/pride-and-prejudice.epub             43% - Chapter 27   |
|                                                                              |
|   Alice's Adventures in Wonderland            Lewis Carroll                  |
|   /home/reader/books/alice.epub                           12% - A Caucus-Race |
|                                                                              |
|   Frankenstein                                Mary Shelley                   |
|   /mnt/books/frankenstein.txt                             Not started         |
|                                                                              |
| ! The Left Hand of Darkness                    Missing                        |
|   /media/books/left-hand-of-darkness.epub                  File not found     |
|                                                                              |
|   No folders are scanned automatically. Open a local TXT, Markdown, or EPUB. |
+------------------------------------------------------------------------------+
| [Enter] Reopen  [o] Open path  [d] Remove  [c] Clear  [?] Help  [q] Quit     |
+------------------------------------------------------------------------------+
```

Behavior:

- Focus begins on the most recent valid or stale entry.
- A stale entry remains selectable and visibly says Missing, Inaccessible, or
  Unsupported rather than repeatedly attempting to open on every redraw.
- Remove and Clear affect recent history only. Their confirmation text says the
  source book and annotations will not be deleted.
- Metadata appears only when trusted. The fallback line remains a recognizable
  filename and path.
- The no-scanning statement is visible in the empty state and discoverable in
  help; the screen never looks like an indexed library.

### Empty Home

```text
+------------------------------------------------------------------------------+
| TermLeaf                                                                     |
+------------------------------------------------------------------------------+
|                                                                              |
|                             No recent books                                  |
|                                                                              |
|                 Open a local TXT, Markdown, or EPUB book.                    |
|                                                                              |
|                   TermLeaf does not scan your folders.                       |
|                                                                              |
|                         [o] Open a book path                                  |
|                                                                              |
+------------------------------------------------------------------------------+
| [?] Help                                                        [q] Quit     |
+------------------------------------------------------------------------------+
```

### Narrow Home

```text
+--------------------------------------+
| TermLeaf                       [?]    |
+--------------------------------------+
| Recent                               |
|                                      |
| > Pride and Prejudice                |
|   Jane Austen - 43%                  |
|                                      |
| ! Left Hand of Darkness              |
|   Missing                            |
+--------------------------------------+
| Enter Open  o Path  q Quit           |
+--------------------------------------+
```

Long paths move into a detail line or temporary details view instead of
horizontal scrolling through the list.

## Open Path

Path entry is a focused text mode. Reading and list shortcuts do not fire while
the field owns focus.

```text
+------------------------------------------------------------------------------+
| Open a local book                                                            |
+------------------------------------------------------------------------------+
|                                                                              |
| Path                                                                         |
| +--------------------------------------------------------------------------+ |
| | /home/reader/books/                                                       | |
| +--------------------------------------------------------------------------+ |
|                                                                              |
| Supported: TXT, Markdown, reflowable EPUB 2 and EPUB 3                       |
| The path is opened only after confirmation. No directory will be scanned.   |
|                                                                              |
| Recent directory: /home/reader/books                                         |
|                                                                              |
+------------------------------------------------------------------------------+
| [Enter] Open  [Esc] Cancel                                      PATH ENTRY   |
+------------------------------------------------------------------------------+
```

Validation errors stay in this view and preserve entered text:

```text
| ! Cannot open /home/reader/books/missing.epub                                |
|   The file does not exist. Check the path or choose another book.            |
```

The final file-picker behavior is platform-dependent and remains outside these
mockups until the open-path implementation is selected. Typing or pasting a path
must always remain available.

## Paged Reader

### Standard Paper Layout

The outer area uses the Paper canvas role and the inner page uses the Paper page
role. Color names appear here only as implementation annotations.

```text
 warm-gray canvas
        +------------------------------------------------------------+
        | Pride and Prejudice                                        |
        |                                                            |
        |                         Chapter 27                         |
        |                                                            |
        |  With no greater events than these in the Longbourn        |
        |  family, and otherwise diversified by little beyond the    |
        |  walks to Meryton, sometimes dirty and sometimes cold,     |
        |  did January and February pass away.                       |
        |                                                            |
        |  March was to take Elizabeth to Hunsford. She had not at   |
        |  first thought very seriously of going thither; but        |
        |  Charlotte, she soon found, was depending on the plan...   |
        |                                                            |
        |                                                            |
        |                                                Page 118    |
        +------------------------------------------------------------+
+------------------------------------------------------------------------------+
| Pride and Prejudice | Ch 27 | Loc 1842 | 43% | PAGED | 10:42 PM | [?]       |
+------------------------------------------------------------------------------+
```

Rules:

- The content viewport, after status and visible frame reservation, defines one
  dynamic page.
- The small page number is optional presentation and never a saved identity.
- The header inside the page may show the title at chapter boundaries but must
  not repeat on every page if it crowds the passage.
- Page movement starts from the current logical anchor, not a visual row index
  retained across relayout.
- Paper removes outer canvas, then inner padding, then page boundary as width
  shrinks.

### Standard Non-Paper Layout

```text
+------------------------------------------------------------------------------+
| Chapter 27                                                                   |
+------------------------------------------------------------------------------+
| With no greater events than these in the Longbourn family, and otherwise     |
| diversified by little beyond the walks to Meryton, sometimes dirty and       |
| sometimes cold, did January and February pass away.                          |
|                                                                              |
| March was to take Elizabeth to Hunsford. She had not at first thought very   |
| seriously of going thither; but Charlotte, she soon found, was depending     |
| on the plan...                                                               |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
|                                                                              |
+------------------------------------------------------------------------------+
| Pride and Prejudice | Ch 27 | Loc 1842 | 43% | PAGED | 10:42 PM | [?]       |
+------------------------------------------------------------------------------+
```

## Continuous Reader

Continuous mode uses the same logical document and styling but moves by visual
rows. It does not add a permanent scrollbar that consumes width. More-content
markers may appear only when they do not conflict with source text.

```text
+------------------------------------------------------------------------------+
| Chapter 27                                                                   |
+------------------------------------------------------------------------------+
| ^                                                                            |
| With no greater events than these in the Longbourn family, and otherwise     |
| diversified by little beyond the walks to Meryton, sometimes dirty and       |
| sometimes cold, did January and February pass away.                          |
|                                                                              |
| March was to take Elizabeth to Hunsford. She had not at first thought very   |
| seriously of going thither; but Charlotte, she soon found, was depending     |
| on the plan...                                                               |
|                                                                              |
| "My dear Eliza!" exclaimed Mrs. Collins, "what a delightful thing this is,   |
| and such a good-humoured handsome man!"                                      |
|                                                                              |
| v                                                                            |
+------------------------------------------------------------------------------+
| Pride and Prejudice | Ch 27 | Loc 1842 | 43% | CONT | 10:42 PM | [?]        |
+------------------------------------------------------------------------------+
```

Switching between Paged and Continuous keeps the first meaningful visible
passage anchored. The mode label changes immediately and is available without
color.

## Compact Reader

```text
+----------------------------------------------------------+
| Chapter 27                                               |
+----------------------------------------------------------+
| With no greater events than these in the Longbourn       |
| family, and otherwise diversified by little beyond the   |
| walks to Meryton, sometimes dirty and sometimes cold,    |
| did January and February pass away.                      |
|                                                          |
| March was to take Elizabeth to Hunsford...               |
|                                                          |
|                                                          |
|                                                          |
|                                                          |
|                                                          |
+----------------------------------------------------------+
| Ch 27 | 43% | PAGED | 10:42 PM | [?]                    |
+----------------------------------------------------------+
```

Narrower layouts remove the header border and shorten status labels before
truncating meaningful passage text:

```text
+--------------------------------------+
| Chapter 27                           |
|                                      |
| With no greater events than these in |
| the Longbourn family, and otherwise  |
| diversified by little beyond the     |
| walks to Meryton...                  |
|                                      |
|                                      |
+--------------------------------------+
| 43%  PAGED  Ch 27  [?]              |
+--------------------------------------+
```

## Link Focus and Text Selection

### Link Focus

Links remain ordinary readable text until the reader enters link-focus mode or
invokes a current-link action defined by the final key map. Focus moves among
visible links in logical reading order and uses underline plus a text status.

```text
+------------------------------------------------------------------------------+
| Further information is available from Project Gutenberg.                    |
|                                         ^^^^^^^^^^^^^^^^^                    |
|                                                                              |
| Destination: https://www.gutenberg.org/ebooks/1342                           |
+------------------------------------------------------------------------------+
| LINK 1 of 2 | [Enter] Follow  [next/prev] Move  [Esc] Return                 |
+------------------------------------------------------------------------------+
```

- Internal links show `INTERNAL LINK` and navigate without a browser prompt.
- External links show `EXTERNAL LINK` and open the confirmation view.
- Resize preserves focus on the same logical link, not its old visual row.
- If the link leaves the viewport, the reader scrolls enough to keep it visible.
- A malformed or unsupported destination remains selectable as text but its
  status says it cannot be opened.

### Text Selection

Highlight and range-note creation begins in a dedicated selection mode. The
logical start is captured first; navigation extends the end by grapheme-safe
logical positions.

```text
+------------------------------------------------------------------------------+
| March was to take Elizabeth to Hunsford. She had not at first thought very   |
| ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^                                  |
| seriously of going thither...                                                |
+------------------------------------------------------------------------------+
| SELECT 42 chars | [extend keys] Adjust | [h] Highlight | [n] Note | Esc Cancel|
+------------------------------------------------------------------------------+
```

Selection rules:

- Focus and range use underline/reverse plus a `SELECT` label, never color only.
- Start and end stay on valid logical/grapheme boundaries through wrapping and
  resize.
- Selection may cross visual rows and supported logical blocks but never create
  an invalid source range.
- Cancel restores the original reading anchor with no annotation.
- Point-note creation is a separate action at the current logical anchor and
  does not pretend a zero-width range is visibly selected.
- Creating a highlight opens Highlight Dialog; creating a range or point note
  opens Note Editor with the attachment type stated in its header.
- Exact keys and allowed cross-block range policy remain Stage 3 decisions and
  must be shared with the test/action registry.

## Search

### Search Entry

Search uses a bottom command band so the passage remains visible and match
context is not lost.

```text
+------------------------------------------------------------------------------+
| Chapter 27                                                                   |
+------------------------------------------------------------------------------+
| ...Charlotte, she soon found, was depending on the plan and she gradually    |
| learned to consider it herself with greater pleasure as well as greater...   |
|                                                                              |
| ...                                                                          |
|                                                                              |
+------------------------------------------------------------------------------+
| / charlotte_                                             [Esc] Cancel        |
+------------------------------------------------------------------------------+
```

The cursor appears only in the input field. Smart-case behavior is explained in
help and may be briefly indicated after submission.

### Search Results

```text
+------------------------------------------------------------------------------+
| Chapter 27                                                                   |
+------------------------------------------------------------------------------+
| ...but Charlotte, she soon found, was depending on the plan and she          |
|     ^^^^^^^^^                                                                |
| gradually learned to consider it herself with greater pleasure...            |
|                                                                              |
+------------------------------------------------------------------------------+
| Search: charlotte | 3 of 14 | smart-case: insensitive | [n] Next [N] Prev    |
+------------------------------------------------------------------------------+
```

ASCII carets illustrate a non-color match cue. Actual rendering may use
background plus underline or reverse. A wrap message is explicit:

```text
| Wrapped to first match | 1 of 14 | [n] Next [N] Prev                         |
```

No-results state:

```text
| No matches for "charlotte_" | [/] Edit search | [Esc] Close                  |
```

### Search History

History is local, bounded by the final state policy, and available from search
entry through a discoverable action.

```text
+------------------------------------------------------------------------------+
| Search history                                                   4 terms     |
+------------------------------------------------------------------------------+
| > charlotte                                                                  |
|   Darcy                                                                      |
|   Hunsford                                                                   |
|   proposal                                                                   |
|                                                                              |
| History belongs to this local TermLeaf state. No cross-book body index exists.|
+------------------------------------------------------------------------------+
| [Enter] Search  [d] Remove term  [c] Clear history  [Esc] Return             |
+------------------------------------------------------------------------------+
```

Clear opens the destructive confirmation pattern and names Search history as the
only data affected. Cancel preserves ordering and the current search field.

## Table of Contents

Wide terminals may use a side panel while retaining passage context:

```text
+------------------------------+-----------------------------------------------+
| Table of contents            | Pride and Prejudice                           |
|                              |                                               |
|   Volume I                   | Chapter 27                                    |
|   Chapter 1                  |                                               |
|   Chapter 2                  | With no greater events than these...          |
|   ...                        |                                               |
| > Chapter 27                 |                                               |
|   Chapter 28                 |                                               |
|   Chapter 29                 |                                               |
|                              |                                               |
|   27 of 61                   |                                               |
+------------------------------+-----------------------------------------------+
| [Enter] Go  [j/k] Move  [Esc] Return                              TOC         |
+------------------------------------------------------------------------------+
```

Standard and narrow terminals use a full-screen temporary list:

```text
+------------------------------------------------------------------------------+
| Table of contents                                              27 of 61      |
+------------------------------------------------------------------------------+
|   Volume I                                                                  |
|   Chapter 1                                                                 |
|   Chapter 2                                                                 |
|   ...                                                                       |
| > Chapter 27                                                                |
|   Chapter 28                                                                |
|   Chapter 29                                                                |
|                                                                             |
+------------------------------------------------------------------------------+
| [Enter] Go  [j/k] Move  [Home/End] First/Last  [Esc] Return                 |
+------------------------------------------------------------------------------+
```

Selecting an entry updates the logical anchor. Cancel returns to the exact
passage that opened the view.

## Bookmarks and Annotations

### Combined Management View

```text
+------------------------------------------------------------------------------+
| Bookmarks and annotations                                      6 items       |
+------------------------------------------------------------------------------+
| Filter: [All] [Bookmarks] [Highlights] [Notes]                                |
|                                                                              |
| > BOOKMARK  Netherfield ball                               Chapter 18         |
|   "Till Elizabeth entered the drawing-room at Netherfield..."                |
|                                                                              |
|   HIGHLIGHT [Olive]                                      Chapter 27         |
|   "March was to take Elizabeth to Hunsford."                                 |
|                                                                              |
|   NOTE                                               Chapter 34              |
|   Compare Darcy's first proposal with the letter.                            |
|   "In an unquiet state of mind..."                                          |
|                                                                              |
|   ! HIGHLIGHT - source changed                              Unresolved        |
|   Original passage preview remains available.                                  |
+------------------------------------------------------------------------------+
| [Enter] Go  [e] Edit  [d] Delete  [f] Filter  [Esc] Return       ANNOTATIONS |
+------------------------------------------------------------------------------+
```

Rules:

- The view contains current-book items only.
- Item type is written in text. Color supplements the `[Olive]` label.
- Unresolved items remain visible with a reason and never silently attach to an
  unrelated passage.
- Delete opens a confirmation naming the one selected item.
- Jump and return behavior follows the final temporary-view stack policy.

### Narrow Management View

```text
+--------------------------------------+
| Annotations                 6 items  |
+--------------------------------------+
| > BOOKMARK                            |
|   Netherfield ball                   |
|   Chapter 18                         |
|                                      |
|   NOTE                               |
|   Compare Darcy's first...           |
+--------------------------------------+
| Enter Go  e Edit  Esc Return         |
+--------------------------------------+
```

## Bookmark Dialog

```text
+------------------------------------------------------------------------------+
|                                                                              |
|                  +----------------------------------------+                  |
|                  | Add bookmark                           |                  |
|                  |                                        |                  |
|                  | Location: Chapter 27, Loc 1842         |                  |
|                  | Passage: "March was to take..."        |                  |
|                  |                                        |                  |
|                  | Name                                   |                  |
|                  | +------------------------------------+ |                  |
|                  | | Hunsford journey_                  | |                  |
|                  | +------------------------------------+ |                  |
|                  |                                        |                  |
|                  | [Enter] Save        [Esc] Cancel       |                  |
|                  +----------------------------------------+                  |
|                                                                              |
+------------------------------------------------------------------------------+
| BOOKMARK NAME                                                                |
+------------------------------------------------------------------------------+
```

Rename uses the same dialog with the current name selected. Empty and over-limit
names follow the final input-limit policy and never create partial state.

## Highlight Dialog

```text
+------------------------------------------------------------------------------+
| Add highlight                                                                |
+------------------------------------------------------------------------------+
| "March was to take Elizabeth to Hunsford."                                   |
|                                                                              |
| Color                                                                        |
| > (*) Olive       Calm emphasis                                              |
|   ( ) Sepia       Warm emphasis                                              |
|   ( ) Ochre       Strong emphasis                                            |
|   ( ) Monochrome  Underline/reverse fallback                                 |
|                                                                              |
| A text label and non-color style identify every choice.                      |
+------------------------------------------------------------------------------+
| [Enter] Save  [j/k] Choose  [Esc] Cancel                     HIGHLIGHT       |
+------------------------------------------------------------------------------+
```

The final accessible color set may be tuned during implementation, but stored
values must be versioned and render in every built-in theme.

## Note Editor

```text
+------------------------------------------------------------------------------+
| Edit note                                          Chapter 34 - Loc 2311     |
+------------------------------------------------------------------------------+
| Passage                                                                      |
| "In an unquiet state of mind, they had been now for two minutes together..." |
|                                                                              |
| Note                                                                         |
| +--------------------------------------------------------------------------+ |
| | Compare Darcy's first proposal with the letter.                          | |
| | Check how the narrator frames Elizabeth's response._                    | |
| |                                                                          | |
| |                                                                          | |
| +--------------------------------------------------------------------------+ |
|                                                                              |
| Plain text only. The source book will not be modified.                       |
+------------------------------------------------------------------------------+
| [Save action] Save  [Esc] Cancel                                  NOTE EDIT |
+------------------------------------------------------------------------------+
```

The exact save key remains part of the final key map. While editing:

- Letter keys insert text rather than invoking reader commands.
- Paste is accepted only within the configured limit.
- Tabs, newlines, control input, and terminal escape bytes follow a safe text
  policy.
- Cancel leaves the stored note unchanged.
- Save failure keeps the editor and text available with an actionable message.

## Theme Selection

The reader can preview a built-in theme for the current session. Persistence is
through TOML configuration; the application must not silently rewrite a human-
edited config file unless an explicit future policy permits it.

```text
+------------------------------------------------------------------------------+
| Theme                                                                        |
+------------------------------------------------------------------------------+
| > (*) Dark             Current session                                      |
|   ( ) Light                                                                 |
|   ( ) High contrast                                                         |
|   ( ) Monochrome                                                            |
|   ( ) Paper            Warm page and responsive canvas                      |
|                                                                              |
| Preview changes the current session without moving the reading passage.      |
| To make it the startup theme, set: theme = "dark" in config.toml             |
+------------------------------------------------------------------------------+
| [Enter] Apply for session  [c] Show config path  [Esc] Cancel    THEME       |
+------------------------------------------------------------------------------+
```

Rules:

- Applying a theme retains logical anchor, active search, selection, annotation,
  reading mode, and temporary-view origin.
- Help shows how to find the platform-native config path.
- `NO_COLOR` and terminal capability may replace the requested palette with the
  documented fallback while retaining the configured theme name.
- Invalid configured names produce one actionable config error and use a safe
  default without overwriting the file.
- A future save-to-config action requires atomic human-config editing semantics
  and is not implied by this first-release chooser.

## External Link Confirmation

The full validated destination must remain visible. Long URLs wrap as text and
do not scroll horizontally out of view.

```text
+------------------------------------------------------------------------------+
| Open external link?                                                          |
+------------------------------------------------------------------------------+
| This will leave TermLeaf and ask the system browser to open:                 |
|                                                                              |
| https://www.gutenberg.org/ebooks/1342                                        |
|                                                                              |
| The book did not open this link while parsing.                               |
|                                                                              |
| [Enter] Open in browser                                                      |
| [Esc]   Cancel and keep the destination visible                              |
+------------------------------------------------------------------------------+
| EXTERNAL LINK - confirmation required                                        |
+------------------------------------------------------------------------------+
```

Unsupported scheme:

```text
+------------------------------------------------------------------------------+
| Link cannot be opened                                                        |
+------------------------------------------------------------------------------+
| javascript:example()                                                         |
|                                                                              |
| This scheme is not supported. The destination remains text only.             |
+------------------------------------------------------------------------------+
| [Esc] Return                                                                 |
+------------------------------------------------------------------------------+
```

No shell command appears in the UI or implementation path. Confirmation passes
one previously validated URL argument to the platform launcher.

### Critical Value Scrolling

Paths, URLs, and diagnostics that exceed available height use a focused wrapped
text viewport. The entire value remains inspectable through vertical movement;
"visible" means reachable without truncation, not simultaneously present in a
screen too small to contain it.

```text
+--------------------------------------+
| External destination       lines 1-6 |
+--------------------------------------+
| https://example.org/a/very/long/path |
| ?query=one%20two&another=value...    |
|                                      |
| v more                               |
+--------------------------------------+
| j/k Scroll  Enter Open  Esc Cancel   |
+--------------------------------------+
```

Copying remains ordinary terminal selection; TermLeaf does not need clipboard
access. Control bytes are escaped before wrapping. Confirmation always launches
the exact validated full value, not a shortened display string.

## Help

Help content is generated from the same action registry used by input handling.
It must not drift into a hand-maintained second key map.

```text
+------------------------------------------------------------------------------+
| Help                                                     Reading mode        |
+------------------------------------------------------------------------------+
| Navigation                         Search and structure                      |
| Up / k        Previous line        /             Search                     |
| Down / j      Next line            n / N         Next / previous match      |
| Page Up       Previous page        [TOC key]     Table of contents           |
| Page Down     Next page                                                    |
| Home / gg     Start                Bookmarks and notes                       |
| End / G       End                  [action key]  Add/manage                  |
|                                                                              |
| Views                              General                                   |
| [mode key]    Paged/continuous     F1 / ?        Help                       |
| Esc           Return/cancel        q             Quit where unambiguous      |
|                                                                              |
| Image fallback: terminal protocol, cell preview, then caption.               |
| Accessibility: all essential actions work from the keyboard; color is not   |
| the only state indicator. See known Unicode and screen-reader limitations.   |
+------------------------------------------------------------------------------+
| [Search/section action] Find help  [Esc] Return                    HELP       |
+------------------------------------------------------------------------------+
```

Provisional labels in square brackets remain until the final key-map and help
navigation decisions are resolved. Searchable help adds a search input and
match count; scannable help adds section focus. The implementation must choose
one exact tested behavior before the view is Complete.

### Required Help Contexts

| Context | Required content |
| --- | --- |
| Recent books | Reopen, open path, remove, clear, stale state, no scanning, help, quit |
| Open path | Text entry, paste limit, submit, cancel, supported formats, no scanning |
| Reader | Line/page/section/document movement, mode, TOC, links, selection, search, annotations, theme, help, quit |
| Link focus | Next/previous link, internal/external distinction, activate, cancel |
| Text selection | Extend, highlight, point/range note, cancel, logical range behavior |
| Search entry | Smart-case, history, clear, submit, cancel, input limit |
| Search results | Next/previous, wrap message, edit query, close |
| TOC | Move, first/last, go, return |
| Annotation list | Filter, go, edit, delete, unresolved item, return |
| Bookmark/highlight dialogs | Field/choice movement, save, validation, cancel |
| Note editing | Plain text, paste, save, cancel, source immutability |
| Theme | Session preview, config persistence, capability fallback, cancel |
| Link confirmation | Full destination, scroll, confirm, cancel, unsupported scheme |
| Error/too small | Recovery action, preserved state, resize behavior |

### Status Glossary

Help links or navigates from every indicator to plain language:

| Indicator | Explanation |
| --- | --- |
| `Ch 27` | Current chapter or section from document structure |
| `Loc 1842` | Stable logical location, not a terminal row |
| `Page 118` | Dynamic page for the current viewport; not saved as identity |
| `43%` | Overall logical reading progress using the finalized Stage 1 formula |
| `PAGED` | Page movement uses the current content viewport |
| `CONT` | Line movement scrolls visual rows through the same logical document |
| Clock | Local display time in the finalized configured/locale-safe format |
| Save pending/failed | Current position has not yet been confirmed durable |
| Search count | Current match and total local matches in the open book |
| Image fallback | Native protocol, cell preview, or caption path currently used |

All contexts are required content even if the final help navigation is still
Blocked between searchable, scannable, or combined behavior.

### Contextual Help During Note Editing

```text
+------------------------------------------------------------------------------+
| Help                                                     Note editing        |
+------------------------------------------------------------------------------+
| Text keys      Insert plain text                                              |
| Enter          New line                                                       |
| Paste          Insert within configured limit                                 |
| [Save action]  Save note                                                      |
| Esc            Cancel editing                                                 |
|                                                                              |
| Reader navigation keys do not run while this editor has focus.               |
+------------------------------------------------------------------------------+
| [Esc] Return to editor                                                        |
+------------------------------------------------------------------------------+
```

## Image States

### Native or Cell Image

ASCII represents the reserved image rectangle, not actual pixel output:

```text
| The illustration showed Alice standing before the little door.               |
|                                                                              |
|          +----------------------------------------------+                    |
|          |                                              |                    |
|          |              rendered image                  |                    |
|          |                                              |                    |
|          +----------------------------------------------+                    |
|          Alice and the small door - 640 x 480                                |
|                                                                              |
| She tried the little golden key in the lock...                                |
```

The image remains in document reading order. Navigating, resizing, switching
pages, changing themes, or falling back must clear stale protocol output.

Native protocol rendering and cell rendering use separate backend tests even
when they reserve the same logical block. The protocol path owns terminal image
IDs and deletion; the cell path owns ordinary cell bounds and redraw.

### Caption Fallback

```text
| +--------------------------------------------------------------------------+ |
| | IMAGE: Alice and the small door                                          | |
| | 640 x 480 - preview unavailable: terminal has no supported image path    | |
| +--------------------------------------------------------------------------+ |
```

### Decode Failure

```text
| +--------------------------------------------------------------------------+ |
| | IMAGE: Map of the journey                                                | |
| | 1200 x 800 - preview unavailable: image data is malformed                | |
| +--------------------------------------------------------------------------+ |
```

Image failures never replace or block surrounding text. Paper changes the frame
and caption roles but does not alter source image pixels by default.

## Loading and Background Work

Opening a typical book should move directly to readable content. A loading view
appears only when work outlives the immediate frame budget.

```text
+------------------------------------------------------------------------------+
| Opening book                                                                 |
+------------------------------------------------------------------------------+
| Pride and Prejudice                                                          |
| /home/reader/books/pride-and-prejudice.epub                                  |
|                                                                              |
| Reading book structure...                                                    |
|                                                                              |
| Text is parsed before unrelated images.                                      |
+------------------------------------------------------------------------------+
| [Esc] Cancel                                             OPENING             |
+------------------------------------------------------------------------------+
```

Do not use an endlessly animated spinner. A static message changes only at
meaningful stages. Cancel invalidates the work generation; a stale completion
cannot replace the current screen.

Image work uses a compact placeholder inside the passage:

```text
| [Image preview is being prepared...]                                         |
```

The placeholder has stable height where practical to avoid moving the passage
after every worker update.

The layout reserves a bounded size from trusted dimensions before dispatch when
possible. If final dimensions or fallback change the block height, relayout
compensates from the first meaningful visible logical anchor so content above the
image does not jump. Placeholder-to-native, cell, caption, and failure
transitions each preserve that anchor and clear obsolete protocol/cell output.

## Errors and Recovery

### Recoverable In-App Error

```text
+------------------------------------------------------------------------------+
| Chapter 27                                                                   |
+------------------------------------------------------------------------------+
| ...current passage remains visible...                                        |
|                                                                              |
| +--------------------------------------------------------------------------+ |
| | ! Could not save reading position                                        | |
| | The previous state file is still available.                              | |
| | Check permissions for the TermLeaf state directory.                      | |
| |                                                                          | |
| | [Enter] Dismiss                                                         | |
| +--------------------------------------------------------------------------+ |
+------------------------------------------------------------------------------+
| Save failed | position not confirmed | [Enter] Dismiss                      |
+------------------------------------------------------------------------------+
```

The message distinguishes failure to persist from failure to read. It never
claims the current position is durable when parent-directory synchronization or
replacement failed.

### Unsupported Book

```text
+------------------------------------------------------------------------------+
| Cannot open this book                                                        |
+------------------------------------------------------------------------------+
| /home/reader/books/atlas.epub                                                |
|                                                                              |
| This EPUB uses fixed-layout pages. TermLeaf reads reflowable EPUB 2 and      |
| EPUB 3 books; it does not render fixed-layout books in the first release.    |
|                                                                              |
| [Enter] Return to recent books                                                |
+------------------------------------------------------------------------------+
| UNSUPPORTED FORMAT                                                           |
+------------------------------------------------------------------------------+
```

Encryption, malformed archive, unsafe resource, invalid encoding, missing file,
and unsupported state-version screens follow the same what/why/next-action
structure. Security-limit errors do not disclose untrusted control bytes or
unrelated private paths.

### Fatal Startup Error

After terminal restoration, print a plain diagnostic without a mock full-screen
border:

```text
TermLeaf could not start.
Reason: terminal initialization failed while enabling raw mode.
Your terminal settings were restored.
Try: run TermLeaf from an interactive terminal.
```

Representative recoverable failures use the same structure:

- Invalid bookmark or note input keeps the field, cursor, and typed text.
- Annotation save failure keeps the editor and selected logical range.
- Unsupported state version leaves the original state file untouched and offers
  startup without destructive migration only if that policy is approved.
- Invalid config names the key and safe fallback without rewriting TOML.
- Browser-launch failure keeps the validated destination inspectable.
- Image/resource errors remain attached to the image block while surrounding
  text stays readable.

## Destructive Confirmations

One reusable pattern covers clearing recents, clearing search history, deleting
an annotation, and removing one recent entry. The safest action owns initial
focus.

```text
+------------------------------------------------------------------------------+
|                                                                              |
|                   +--------------------------------------+                   |
|                   | Remove recent entry?                 |                   |
|                   |                                      |                   |
|                   | Pride and Prejudice                  |                   |
|                   |                                      |                   |
|                   | The source book, saved position, and |                   |
|                   | annotations will not be deleted.     |                   |
|                   |                                      |                   |
|                   | > [Esc] Cancel    [confirm] Remove   |                   |
|                   +--------------------------------------+                   |
|                                                                              |
+------------------------------------------------------------------------------+
| CONFIRM REMOVE                                                               |
+------------------------------------------------------------------------------+
```

Rules:

- Default focus is Cancel. Enter on initial focus does not delete data.
- Confirmation names the exact item or collection and every type of related data
  that remains untouched.
- Compact/narrow layout becomes full-screen with the same wording and focus.
- Cancel restores the same list row and scroll offset.
- Confirm runs once despite key repeat and reports atomic save failure without
  pretending deletion succeeded.
- Source-file deletion is never offered by these first-release dialogs.

## Terminal Too Small

```text
+--------------------------------------+
| Terminal too small                   |
|                                      |
| TermLeaf needs more room to show     |
| readable text and essential controls.|
|                                      |
| Resize the terminal or press q.      |
+--------------------------------------+
```

At even smaller dimensions, render only complete clipped-safe lines:

```text
Terminal too small
Resize or press q
```

No parser, position, or state reset occurs. When usable space returns, the exact
prior logical anchor and temporary-view context recover.

The state is mode-aware and non-destructive:

- Text-entry buffers, cursor positions, selections, dialog focus, search results,
  unsaved notes, and confirmation targets remain in application state.
- Printable keys are not interpreted while content cannot be shown safely.
- Resize events remain active. A final dedicated quit/termination action may
  exit; literal `q` is not universally treated as quit while a text mode is
  suspended.
- At zero or one cell, rendering emits only complete safe output that fits, or
  no cells, and waits for resize/termination.
- Recovery returns to the exact suspended mode before accepting ordinary input.

## Status Line Rules

Status fields have semantic priority. The final exact collapse order is resolved
with the status test decision, but it must preserve this intent:

| Priority | Field | Reason |
| ---: | --- | --- |
| 1 | Temporary error, warning, confirmation, or pending-save message | Current action and data safety |
| 2 | Reading mode | Changes navigation meaning |
| 3 | Progress percentage or logical location | Durable orientation |
| 4 | Help hint | Keeps keyboard discovery available on compact screens |
| 5 | Current chapter | Structural orientation |
| 6 | Book title | Identity when space allows |
| 7 | Dynamic page | Useful but layout-dependent |
| 8 | Clock | Convenient and first to disappear |

Illustrative widths:

```text
Wide:     Pride and Prejudice | Ch 27 | Loc 1842 | Page 118 | 43% | PAGED | 10:42 PM | [?]
Standard: Pride and Prejudice | Ch 27 | 43% | PAGED | 10:42 PM | [?]
Compact:  Ch 27 | 43% | PAGED | 10:42 PM | [?]
Narrow:   43%  PAGED  Ch 27  [?]
Message:  Save failed: previous state retained | [Enter] Details
```

Requirements:

- A fake clock drives render tests.
- Dynamic page may change after resize; logical location and percentage do not
  change merely because layout changed.
- Messages live for deterministic ticks or explicit dismissal, not wall-clock
  sleeps in tests.
- The status line never causes the content viewport to change height while only
  its fields collapse.
- Stage 1 must define logical location units, percentage numerator/denominator
  and rounding, dynamic page calculation, clock format, omission rules, and exact
  collapse transitions before `STATUS-005` can pass.

## Theme Roles

Mockups refer to semantic roles rather than embedding colors in widgets:

| Role | Use |
| --- | --- |
| Canvas | Space outside the reading page or primary panel |
| Surface | Page, list, editor, and dialog background |
| Text | Primary content and essential status |
| Secondary | Metadata and less important hints |
| Accent | Active heading, focus, and selected control |
| Link | Underlined openable destination |
| Selection | Selected logical range plus non-color cue |
| Search match | Search range plus underline/reverse cue |
| Warning | Warning label and message, never color alone |
| Error | Error label and actionable failure text |
| Annotation | Named annotation color plus text/type cue |

Paper uses the exact initial palette and contrast floor in `project_plan.md`.
High-contrast and monochrome themes prioritize state distinction over decorative
similarity. `NO_COLOR` uses terminal defaults and text attributes; it does not
promise contrast for an unknown user palette.

## Focus and Input

| Mode | Focus target | Text input | Reader commands |
| --- | --- | --- | --- |
| Recent books | One recent row | No | List actions only |
| Open path | Path field | Yes | Suppressed except cancel/submit |
| Reader | Logical reading anchor | No | Enabled |
| Link focus | One logical link | No | Link movement/activate/cancel only |
| Text selection | One logical range endpoint | No | Selection extension/actions only |
| Search entry | Search field | Yes | Suppressed except cancel/submit |
| Search history | One history term | No | History actions only |
| Search results | Current result/passage | No | Result and reader navigation |
| TOC | One TOC row | No | List navigation only |
| Annotation list | One item | No | List and item actions |
| Bookmark name | Name field | Yes | Suppressed except cancel/submit |
| Highlight choice | One color option | No | Dialog navigation only |
| Note editor | Note field | Yes | Suppressed except editor actions |
| Theme selection | One theme option | No | Preview/apply/cancel only |
| Link confirmation | Confirm or cancel action | No | Suppressed except dialog actions |
| Help | Help section/search target | Decision-dependent | Help navigation only |
| Error dialog | Recovery action | No | Suppressed except dialog actions |

Focus is visible through more than color. On return, focus goes to the invoking
row, result, passage, or field unless that item no longer exists.

## Overlay Rules

- Use overlays only when the underlying passage gives useful context.
- Clear the overlay rectangle before rendering it so old cells do not leak.
- Dim or restyle the background only when terminal capability permits; do not
  make background text unreadable in monochrome.
- Dialog width is bounded by content and viewport. Long URLs and paths wrap.
- A compact or narrow terminal turns the overlay into a full-screen temporary
  view instead of clipping it.
- Nested overlays are avoided. A delete confirmation replaces the list action
  layer and returns to the same row on cancel.
- Escape cancels or returns one level where the final key map permits it.
- Fatal errors do not remain inside an alternate-screen overlay after cleanup.

## Accessibility

- Every represented action must be keyboard-reachable.
- Type, state, warning, focus, selection, and match information has a text or
  attribute cue independent of color.
- The application does not rely on border shape alone; headings and labels name
  every temporary view.
- Help describes known terminal and screen-reader limitations honestly.
- Reduced redraw is the default. No decorative animation, blinking, or noisy
  paper texture is introduced.
- High-contrast, monochrome, terminal-default, and `NO_COLOR` render paths cover
  every mockup state.
- Plain startup errors remain useful outside the full-screen UI.
- Screen-reader usability is unverified and is not a compatibility claim; the
  automated contract covers keyboard paths, textual cues, stable output, and
  non-color distinctions.

## Implementation Guidance

### State Before Widgets

Represent the active view and focus explicitly in application state. A useful
shape may resemble:

```text
View
  RecentBooks
  OpenPath
  Reader
  LinkFocus
  TextSelection
  SearchEntry
  TableOfContents
  AnnotationList
  BookmarkDialog
  HighlightDialog
  NoteEditor
  LinkConfirmation
  Help
  RecoverableError
  TooSmall
```

The exact Rust enum may differ. It must prevent impossible combinations, such as
search input and note editing owning focus simultaneously.

### Render Pipeline

1. Derive responsive class and content rectangle from terminal dimensions.
2. Derive semantic theme roles from configured theme and color capability.
3. Derive visible logical rows from reader anchor and content rectangle.
4. Render base screen into a cell model.
5. Render one temporary view or overlay when active.
6. Render status/message band without changing content height.
7. Place cursor only for active text input.
8. Diff/redraw through Ratatui/Crossterm and update image protocol state safely.

Core document, layout, reader, and persistence modules must not depend on these
widgets. UI converts their typed state into cells; it does not own logical
positions, source ranges, parsing, or durable state.

### Component Boundaries

Prefer a small set of state-driven render functions:

```text
render_recent_books
render_reader
render_status
render_search
render_toc
render_annotations
render_text_editor
render_confirmation
render_help
render_error
```

Do not create a framework of one-line widgets. Extract a component when it owns
a reusable layout rule, semantic role, focus behavior, or independent render
test. Input handling dispatches application actions and does not mutate widget
internals directly.

### Content Safety

- Sanitize control characters before placing untrusted title, path, note, URL,
  metadata, caption, or error text into terminal cells.
- Truncation preserves safe grapheme boundaries and exposes full critical values
  in a detail or confirmation view.
- Never interpolate a displayed URL into a shell command.
- Render notes as plain text.
- Do not load remote resources to complete a screen.
- Do not decode invisible images merely because a mockup reserves an image area.

## Render Test Mapping

| Mockup area | Primary case families |
| --- | --- |
| Recent books and empty state | `RECENT`, `PRIV`, `RENDER` |
| Paged/continuous/compact reader | `LAY`, `NAV`, `STATUS`, `RENDER` |
| Search input and results | `SEARCH`, `NAV`, `RENDER` |
| TOC | `NAV`, `EPUB`, `HELP`, `RENDER` |
| Bookmarks and annotations | `ANN`, `STATE`, `A11Y`, `RENDER` |
| Text dialogs and editor | `KEY`, `ANN`, `STATE`, `ERR` |
| Link confirmation | `LINK`, `PRIV`, `A11Y`, `RENDER` |
| Help | `HELP`, `KEY`, `A11Y`, `RENDER` |
| Image success and fallback | `IMG`, `THEME`, `CON`, `RENDER` |
| Loading/background work | `CON`, `PERF`, `ERR`, `RENDER` |
| Errors and too-small state | `ERR`, `TERM`, `LAY`, `RENDER` |
| Theme variants | `THEME`, `A11Y`, `RENDER` |

Each screen receives direct assertions for required text, focus, bounds, source
mapping, non-color cues, and logical anchor before a deterministic baseline can
change. A baseline update alone does not establish conformance.

## Open UI Decisions

These decisions are intentionally not hidden inside mockups:

| Decision | Stage | Affected views |
| --- | --- | --- |
| Exact conflict-free key map and multikey timing | 1 | Reader, lists, editors, help |
| Minimum usable terminal dimensions and derived responsive thresholds | 1 | Every full-screen state |
| Exact status collapse order and message lifetime | 1 | Reader and temporary messages |
| Logical location, percentage, dynamic page, rounding, and clock formulas | 1 | Reader status and help glossary |
| Format detection and non-TTY behavior | 0-1 | Startup and open path |
| Open-path platform picker versus typed/pasted path only | 1 | Recent books and open path |
| Theme action binding and session/config persistence details | 1 | Reader, theme view, help |
| Loading-view threshold and stage messages | 1-2 | Startup, structured books, images |
| Search empty/control/length policy | 3 | Search input |
| Search history capacity, remove, and clear-confirmation details | 3 | Search history and state |
| Searchable, scannable, or combined help navigation | 3 | Help |
| Text selection keys, cross-block range policy, and point/range note action | 3 | Reader, highlight, note editor |
| Final accessible highlight palette and stored identifiers | 3 | Highlight dialog and themes |
| Bookmark, note, path, paste, URL, recents, and annotation limits | 0-3 | All text/input and persisted views |
| Annotation relocation and return-stack behavior | 3-4 | Annotation list and reader |
| External scheme allowlist and URL length | 4 | Link confirmation |
| Supported native OS/terminal/version rows | 0 and 5 | Release evidence |

Until resolved in the product plan and test registry, an implementation may
prototype these details but must not mark their cases Passing or present one
prototype as a locked contract.

## Phase Ownership

The UI work fits the existing six phases:

| Phase | UI responsibility |
| --- | --- |
| 0. Rust foundation | Application view state, action dispatch, terminal guard, base screen shell, test backend, case registry, and profile manifests |
| 1. Plain-text reading loop | Paged/continuous reader, responsive classes, status foundation, core key map, all built-in themes, Paper layout, help skeleton, errors, and too-small state |
| 2. Structured books and images | TOC, semantic code/table rendering, image placement, loading, protocol/cell/caption states, and safe resource errors |
| 3. Dependable reading | Recent books, search, bookmarks, highlights, notes, complete help, configuration/state feedback, and accessibility views |
| 4. Product refinement | Metadata details, annotation recovery, external links, scripted interaction, automated accessibility, performance, and guidance refinement |
| 5. Release | Automated native-runner/PTY matrix, package/install journeys, generated captures, known limitations, and release documentation |

No seventh phase is needed. The quality and test standards add entry and exit
evidence to every phase; the mockups clarify work already promised by the first-
release contract.

## Mockup Completion Rules

A represented screen is implementation-complete only when:

- Required information and actions match the product contract.
- Wide, standard, compact, narrow, and below-minimum behavior is defined where
  the screen can appear.
- Focus, cursor, cancel, return, loading, empty, success, and failure states are
  deterministic.
- Theme roles and non-color cues pass direct assertions.
- Untrusted text is terminal-safe and critical values remain inspectable.
- Logical reading position survives overlays, resize, theme, and mode changes.
- The mapped stable test cases pass at required layers and environments.
- Native evidence supports any terminal, image, keyboard, accessibility, or
  platform claim.
- Documentation, help, trackers, and test reports describe the delivered result.
