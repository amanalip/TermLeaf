//! Bounded, one-shot terminal graphics capability probing.
//!
//! Crossterm owns terminal input for the full session. Probe responses that it
//! exposes as key events are reconstructed here, while unrelated events are
//! queued for the ordinary event loop. No second stdin reader races that owner.

use std::{
    collections::VecDeque,
    io::{self, Write},
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::terminal_image::{CapabilityEvidence, ImageCapabilities};

/// One shared deadline prevents three absent protocols from multiplying delay.
pub const PROBE_TIMEOUT: Duration = Duration::from_millis(250);
/// Total response bytes retained during the one-shot probe.
pub const PROBE_RESPONSE_LIMIT: usize = 4096;
/// Maximum bytes accepted for one APC or DCS response frame.
pub const PROBE_FRAME_LIMIT: usize = 1024;

const KITTY_QUERY_PREFIX: &[u8] = b"\x1b_Gi=";
const KITTY_QUERY_SUFFIX: &[u8] = b",s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";
/// `TN` is requested through XTGETTCAP. Only a returned terminal name that
/// explicitly identifies a Sixel variant is accepted.
pub const SIXEL_QUERY: &[u8] = b"\x1bP+q544e\x1b\\";
/// iTerm2 documents this extended-device-attributes query and DCS response.
pub const ITERM2_QUERY: &[u8] = b"\x1b[>q";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbeOutcome {
    Positive,
    Negative,
    Malformed,
    Partial,
    OverLimit,
    Absent,
}

impl ProbeOutcome {
    const fn evidence(self) -> CapabilityEvidence {
        match self {
            Self::Positive => CapabilityEvidence::Positive,
            Self::Negative => CapabilityEvidence::Negative,
            Self::Malformed | Self::Partial | Self::OverLimit => CapabilityEvidence::Malformed,
            Self::Absent => CapabilityEvidence::Absent,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbeReport {
    pub kitty: ProbeOutcome,
    pub sixel: ProbeOutcome,
    pub iterm2: ProbeOutcome,
}

impl Default for ProbeReport {
    fn default() -> Self {
        Self {
            kitty: ProbeOutcome::Absent,
            sixel: ProbeOutcome::Absent,
            iterm2: ProbeOutcome::Absent,
        }
    }
}

impl ProbeReport {
    #[must_use]
    pub const fn capabilities(self) -> ImageCapabilities {
        ImageCapabilities {
            kitty: self.kitty.evidence(),
            sixel: self.sixel.evidence(),
            iterm2: self.iterm2.evidence(),
            true_color: CapabilityEvidence::Absent,
            ansi256: CapabilityEvidence::Absent,
        }
    }
}

#[derive(Debug, Default)]
pub struct ProbeResult {
    pub report: ProbeReport,
    pub queued_events: VecDeque<Event>,
}

#[derive(Debug)]
struct ObservedEvent {
    event: Event,
    start: usize,
    end: usize,
}

/// Sends all protocol queries once and collects responses under one deadline.
///
/// # Errors
///
/// Returns terminal output or Crossterm input errors. Callers may safely treat
/// an error as absent evidence and retain the caption/cell fallback.
pub fn probe_crossterm(output: &mut impl Write) -> io::Result<ProbeResult> {
    let kitty_id = std::process::id().max(1);
    write_queries(output, kitty_id)?;

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut bytes = Vec::with_capacity(256);
    let mut observed = Vec::new();
    let mut over_limit = false;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !event::poll(remaining)? {
            break;
        }
        let item = event::read()?;
        let start = bytes.len();
        if let Some(encoded) = event_bytes(&item) {
            if bytes.len().saturating_add(encoded.len()) > PROBE_RESPONSE_LIMIT {
                over_limit = true;
            } else {
                bytes.extend_from_slice(&encoded);
            }
        }
        observed.push(ObservedEvent {
            event: item,
            start,
            end: bytes.len(),
        });
        if all_complete(&bytes, kitty_id) {
            break;
        }
    }

    let (mut report, consumed) = analyze(&bytes, kitty_id);
    if over_limit {
        for outcome in [&mut report.kitty, &mut report.sixel, &mut report.iterm2] {
            if *outcome == ProbeOutcome::Absent {
                *outcome = ProbeOutcome::OverLimit;
            }
        }
    }
    let queued_events = observed
        .into_iter()
        .filter(|item| {
            item.start == item.end
                || !consumed
                    .iter()
                    .any(|range| item.start < range.end && item.end > range.start)
        })
        .map(|item| item.event)
        .collect();
    Ok(ProbeResult {
        report,
        queued_events,
    })
}

fn write_queries(output: &mut impl Write, kitty_id: u32) -> io::Result<()> {
    output.write_all(KITTY_QUERY_PREFIX)?;
    write!(output, "{kitty_id}")?;
    output.write_all(KITTY_QUERY_SUFFIX)?;
    output.write_all(SIXEL_QUERY)?;
    output.write_all(ITERM2_QUERY)?;
    output.flush()
}

fn event_bytes(item: &Event) -> Option<Vec<u8>> {
    let Event::Key(key) = item else {
        return None;
    };
    match key.code {
        KeyCode::Esc if key.modifiers.is_empty() => Some(vec![0x1b]),
        KeyCode::Char(character)
            if key
                .modifiers
                .difference(KeyModifiers::SHIFT | KeyModifiers::ALT)
                .is_empty() =>
        {
            let mut encoded = Vec::with_capacity(5);
            if key.modifiers.contains(KeyModifiers::ALT) {
                encoded.push(0x1b);
            }
            let mut utf8 = [0; 4];
            encoded.extend_from_slice(character.encode_utf8(&mut utf8).as_bytes());
            Some(encoded)
        }
        _ => None,
    }
}

fn all_complete(bytes: &[u8], kitty_id: u32) -> bool {
    let (report, _) = analyze(bytes, kitty_id);
    [report.kitty, report.sixel, report.iterm2]
        .into_iter()
        .all(|outcome| !matches!(outcome, ProbeOutcome::Absent | ProbeOutcome::Partial))
}

#[derive(Clone, Debug)]
struct ResponseRange {
    start: usize,
    end: usize,
}

fn analyze(bytes: &[u8], kitty_id: u32) -> (ProbeReport, Vec<ResponseRange>) {
    let mut report = ProbeReport::default();
    let mut consumed = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some((start, kind)) = next_frame(bytes, cursor) else {
            break;
        };
        let Some(relative_end) = bytes[start + 2..]
            .windows(2)
            .position(|part| part == b"\x1b\\")
        else {
            let outcome = if bytes.len() - start > PROBE_FRAME_LIMIT {
                ProbeOutcome::OverLimit
            } else {
                ProbeOutcome::Partial
            };
            apply_outcome(&mut report, kind, outcome);
            consumed.push(ResponseRange {
                start,
                end: bytes.len(),
            });
            break;
        };
        let end = start + 2 + relative_end + 2;
        let frame = &bytes[start..end];
        let outcome = if frame.len() > PROBE_FRAME_LIMIT {
            ProbeOutcome::OverLimit
        } else {
            match kind {
                FrameKind::Kitty => parse_kitty(frame, kitty_id),
                FrameKind::Dcs => {
                    if frame.starts_with(b"\x1bP>|iTerm2 ") {
                        parse_iterm2(frame)
                    } else {
                        parse_sixel(frame)
                    }
                }
            }
        };
        if kind == FrameKind::Dcs && frame.starts_with(b"\x1bP>|iTerm2 ") {
            report.iterm2 = outcome;
        } else {
            apply_outcome(&mut report, kind, outcome);
        }
        consumed.push(ResponseRange { start, end });
        cursor = end;
    }
    (report, consumed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameKind {
    Kitty,
    Dcs,
}

fn next_frame(bytes: &[u8], from: usize) -> Option<(usize, FrameKind)> {
    let kitty = find(bytes, from, b"\x1b_G").map(|index| (index, FrameKind::Kitty));
    let dcs = find(bytes, from, b"\x1bP").map(|index| (index, FrameKind::Dcs));
    match (kitty, dcs) {
        (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
        (Some(found), None) | (None, Some(found)) => Some(found),
        (None, None) => None,
    }
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|part| part == needle)
        .map(|index| from + index)
}

fn apply_outcome(report: &mut ProbeReport, kind: FrameKind, outcome: ProbeOutcome) {
    match kind {
        FrameKind::Kitty => report.kitty = outcome,
        FrameKind::Dcs => report.sixel = outcome,
    }
}

fn parse_kitty(frame: &[u8], kitty_id: u32) -> ProbeOutcome {
    let expected = format!("\x1b_Gi={kitty_id};");
    if !frame.starts_with(expected.as_bytes()) || !frame.ends_with(b"\x1b\\") {
        return ProbeOutcome::Malformed;
    }
    let status = &frame[expected.len()..frame.len() - 2];
    if status == b"OK" {
        ProbeOutcome::Positive
    } else if !status.is_empty() && status.iter().all(u8::is_ascii_graphic) {
        ProbeOutcome::Negative
    } else {
        ProbeOutcome::Malformed
    }
}

fn parse_sixel(frame: &[u8]) -> ProbeOutcome {
    const POSITIVE: &[u8] = b"\x1bP1+r544e=";
    if frame == b"\x1bP0+r544e\x1b\\" || frame == b"\x1bP0+r\x1b\\" {
        return ProbeOutcome::Negative;
    }
    if !frame.starts_with(POSITIVE) || !frame.ends_with(b"\x1b\\") {
        return ProbeOutcome::Malformed;
    }
    let encoded = &frame[POSITIVE.len()..frame.len() - 2];
    let Some(name) = decode_hex(encoded) else {
        return ProbeOutcome::Malformed;
    };
    let Ok(name) = std::str::from_utf8(&name) else {
        return ProbeOutcome::Malformed;
    };
    if name
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|part| part.eq_ignore_ascii_case("sixel"))
    {
        ProbeOutcome::Positive
    } else {
        ProbeOutcome::Negative
    }
}

fn parse_iterm2(frame: &[u8]) -> ProbeOutcome {
    const PREFIX: &[u8] = b"\x1bP>|iTerm2 ";
    if !frame.starts_with(PREFIX) || !frame.ends_with(b"\x1b\\") {
        return ProbeOutcome::Malformed;
    }
    let version = &frame[PREFIX.len()..frame.len() - 2];
    if !version.is_empty()
        && version
            .iter()
            .all(|byte| byte.is_ascii_digit() || *byte == b'.')
        && version
            .split(|byte| *byte == b'.')
            .all(|part| !part.is_empty())
    {
        ProbeOutcome::Positive
    } else {
        ProbeOutcome::Malformed
    }
}

fn decode_hex(encoded: &[u8]) -> Option<Vec<u8>> {
    if !encoded.len().is_multiple_of(2) || encoded.len() > 256 {
        return None;
    }
    encoded
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            u8::try_from((high << 4) | low).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: u32 = 77;

    #[test]
    fn img_017_protocol_response_table_requires_complete_correlated_evidence() {
        let rows: &[(&[u8], ProbeReport)] = &[
            (
                b"\x1b_Gi=77;OK\x1b\\",
                ProbeReport {
                    kitty: ProbeOutcome::Positive,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1b_Gi=76;OK\x1b\\",
                ProbeReport {
                    kitty: ProbeOutcome::Malformed,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1b_Gi=77;EINVAL\x1b\\",
                ProbeReport {
                    kitty: ProbeOutcome::Negative,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1b_Gi=77;OK",
                ProbeReport {
                    kitty: ProbeOutcome::Partial,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1bP1+r544e=787465726d2d736978656c\x1b\\",
                ProbeReport {
                    sixel: ProbeOutcome::Positive,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1bP1+r544e=787465726d2d323536636f6c6f72\x1b\\",
                ProbeReport {
                    sixel: ProbeOutcome::Negative,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1bP>|iTerm2 3.5.10\x1b\\",
                ProbeReport {
                    iterm2: ProbeOutcome::Positive,
                    ..ProbeReport::default()
                },
            ),
            (
                b"\x1bP>|iTerm2 latest\x1b\\",
                ProbeReport {
                    iterm2: ProbeOutcome::Malformed,
                    ..ProbeReport::default()
                },
            ),
        ];
        for (input, expected) in rows {
            assert_eq!(analyze(input, ID).0, *expected, "input={input:?}");
        }
    }

    #[test]
    fn img_017_multiple_responses_keep_locked_protocol_precedence_available() {
        let input = b"q\x1bP>|iTerm2 3.5.10\x1b\\x\x1bP1+r544e=787465726d2d736978656c\x1b\\\x1b_Gi=77;OK\x1b\\";
        let (report, ranges) = analyze(input, ID);
        assert_eq!(report.kitty, ProbeOutcome::Positive);
        assert_eq!(report.sixel, ProbeOutcome::Positive);
        assert_eq!(report.iterm2, ProbeOutcome::Positive);
        assert_eq!(ranges.len(), 3);
    }

    #[test]
    fn img_017_frame_and_total_limits_are_inclusive() {
        let mut at_limit = b"\x1b_Gi=77;".to_vec();
        at_limit.resize(PROBE_FRAME_LIMIT - 2, b'x');
        at_limit.extend_from_slice(b"\x1b\\");
        assert_ne!(analyze(&at_limit, ID).0.kitty, ProbeOutcome::OverLimit);

        let mut over = at_limit;
        over.insert(over.len() - 2, b'x');
        assert_eq!(analyze(&over, ID).0.kitty, ProbeOutcome::OverLimit);
        assert_eq!(PROBE_RESPONSE_LIMIT, 4 * PROBE_FRAME_LIMIT);
    }

    #[test]
    fn query_packet_is_written_once_in_protocol_order() -> io::Result<()> {
        let mut output = Vec::new();
        write_queries(&mut output, ID)?;
        assert_eq!(
            output,
            b"\x1b_Gi=77,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1bP+q544e\x1b\\\x1b[>q"
        );
        Ok(())
    }
}
