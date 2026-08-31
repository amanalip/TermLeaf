# Manual Test Procedures

**Last updated:** August 30, 2026

These procedures cover the catalog cases whose evidence requires a human at a
real terminal. Automated PTY-layer equivalents run in CI on every required
environment row; the steps below record what automation cannot see: real
terminal fonts, input-method behavior, and host rendering.

Each procedure ends with a recording block. A case becomes Passing only when
a tester records tester, platform, terminal, date, and observed result for
every step, per the completion rules in `testcases.md`.

## Recording template

```text
Case ID:
Tester:
Platform and OS version:
Terminal and version:
Session (direct/SSH/tmux):
Date:
Steps observed:
Result (pass/fail per oracle):
Limitations noted:
```

## KEY-001: reader keys fire exactly once on a native row

Automated equivalent: `tests/pty_native.rs::key_001_reader_keys_navigate_help_and_quit_inside_a_pty`
runs on every Required `ENV-*` row in the hosted `native-pty` job.

Manual steps (any Deferred native terminal row):

1. Open a multi-page plain text book: `termleaf book.txt`.
2. Press Down then Up; the status location returns to its start value.
3. Press PageDown then PageUp; the page anchor returns to its start value.
4. Press End; the view jumps to the final passage and 100 percent.
5. Press Home; the view returns to the first line.
6. Press F1; help overlays without moving the status location.
7. Press Escape; help closes on the same passage.
8. Press q; the shell prompt returns with no damaged state.
9. Repeat steps 2-8 inside tmux where practical.

Oracle: each key produces its documented action exactly once; the help view
shows the same binding labels (`F1`, `PgDn`, and so on); the terminal is
restored after exit.

## KEY-002: Ctrl-B and Ctrl-F under flow control

Automated equivalent: `tests/pty_native.rs::key_002_flow_control_keys_page_without_colliding_in_a_pty`.
Raw mode disables IXON inside TermLeaf, so Ctrl-B/Ctrl-F arrive as page keys.

Manual steps:

1. Open a book with software flow control enabled (`stty ixon` beforehand).
2. Press Ctrl-F repeatedly; pages advance without pausing input.
3. Press Ctrl-B; pages move back by the same distance.
4. Press plain b and f; nothing happens (no collision).
5. Restore your original `stty` settings afterward.

Oracle: paging is reliable and plain letters stay inert; no key repeat storm
or lost input occurs.

## KEY-005: AltGr and non-Latin entry in search and notes (Phase 3)

Blocked forward to Phase 3 by DD-026: search entry and note editors do not
exist yet. Execute this procedure when they land, before claiming their
behavior:

1. Open a book; enter search with `/`.
2. Type AltGr characters and non-Latin words; confirm exact insertion.
3. Cancel search with Escape; create a note and repeat the typing check.
4. Confirm no reading command fires from either buffer.

## KEY-006: Escape versus Alt ambiguity on a real terminal

Automated equivalent: `tests/pty_native.rs::key_006_escape_alt_ambiguity_and_ctrl_c_stay_safe_in_a_pty`.

Manual steps:

1. Open help with `?`.
2. Press Escape alone; help closes.
3. Open help again; press a chord such as Alt-x quickly; help stays open.
4. Hold Escape during rapid navigation; Back never corrupts the prefix map.

Oracle: lone Escape always means Back; Alt chords never act as Back plus a
letter; Ctrl-C exits cleanly from any Phase 1 view.

## LAY-013: Unicode placement on a native row (font-dependent half)

Render-layer cell claims are automated in
`tests/render.rs::lay_013_unicode_placement_claims_match_support_limits`;
the font-dependent observations below belong to the release matrix:

1. Render combining marks, CJK, ambiguous-width characters, ZWJ emoji,
   flags, and skin tones from `tests/render.rs` fixtures.
2. Record terminal font, font size, and any width overrides.
3. Note cursor placement after wide glyphs and any double-width mismatches.
4. Document every mismatch as a known limitation instead of a claim.

## LAY-014: right-to-left samples on a native row

Integration evidence is automated in
`tests/document_io.rs::lay_014_right_to_left_samples_stay_bounded_and_logical`.

Manual observation (informational until bidi support is decided):

1. Open an Arabic or Hebrew sample.
2. Verify logical-order rendering does not panic or corrupt the screen.
3. Record that visual reordering is not performed; search and annotations
   over reordered text are unsupported.

## IMG-018: native image protocol lifecycle

Automated supporting evidence:
`tests/pty_native.rs::img_018_kitty_display_replace_and_shutdown_cleanup_over_pty`
and
`tests/pty_native.rs::img_018_sixel_and_iterm2_emit_only_the_selected_protocol_over_pty`.
PTY acceptance proves bytes and lifecycle ordering, not that a terminal displayed
the pixels correctly.

Prerequisites:

1. Select one finalized direct terminal/version for the protocol under test.
2. Record OS, terminal version, session type, cell geometry, pixel geometry, and
   whether a multiplexer is present.
3. Use a Markdown or EPUB fixture with text before and after two distinct images,
   one corrupt image, and enough text to scroll images outside the viewport.
4. Keep a raw terminal-output capture when the terminal provides a safe capture
   method; do not publish private paths from the capture.

Procedure, repeated independently for each claimed Linux protocol (Kitty and
Sixel for the current scope; iTerm2 is not a Linux release claim):

1. Start TermLeaf with the fixture and verify exactly one capability-query packet
   occurs before the first frame.
2. Verify the first image appears at its caption, has the expected colors and
   aspect ratio, and does not overwrite surrounding text.
3. Leave the frame unchanged for two seconds; verify the image does not flicker
   or retransmit visibly.
4. Resize wider, narrower, and back. Verify replacement stays aligned with its
   caption and stale pixels are removed.
5. Scroll the image fully outside the viewport, then back inside. Verify stale
   pixels disappear and the current image returns once at the correct location.
6. Navigate to another section and back. Verify the current generation replaces
   any old placement rather than displaying a late stale result.
7. Reach the second image and verify it replaces or coexists according to the
   visible layout without reusing the wrong logical image.
8. Reach the corrupt image. Verify a readable caption and short failure reason
   appear, surrounding text remains usable, and no partial native image remains.
9. For Sixel, repeat once with unavailable pixel geometry and verify a caption
   fallback rather than cell counts being treated as pixels.
10. Quit while an image is visible, then repeat while an image is loading. Verify
    bounded exit, stale-image cleanup, cursor visibility, input echo, alternate
    screen restoration, and an undamaged shell prompt.
11. Record observed display, resize, scroll, replacement, failure, and cleanup
    results separately. A pass for one protocol does not promote another.

Oracle: the terminal accepts only the selected protocol's framing; images remain
aligned through resize and navigation; stale placements are removed; failures
fall back to text; and exit restores the terminal. Any missing observation,
unsupported geometry, multiplexer rewrite, or cleanup defect is a failed or
excluded tuple, not a partial compatibility claim.

## Execution status

| Case | Automated PTY/render layer | Manual procedure |
| --- | --- | --- |
| KEY-001 | Passing locally; hosted rows recorded with CI runs | Pending release matrix execution |
| KEY-002 | Passing locally; hosted rows recorded with CI runs | Pending release matrix execution |
| KEY-005 | Not applicable until Phase 3 features land | Blocked forward by DD-026 |
| KEY-006 | Passing locally; hosted rows recorded with CI runs | Pending release matrix execution |
| LAY-013 | Cell-level claims passing | Font-dependent half pending release matrix |
| LAY-014 | Integration journey passing | Informational observation pending release matrix |
| IMG-018 | Deterministic PTY byte/lifecycle support passing locally | Linux Kitty and Sixel native observations not executed; iTerm2 outside scope |
