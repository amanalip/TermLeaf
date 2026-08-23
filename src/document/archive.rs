//! Bounded ZIP preflight in front of EPUB semantic parsing.
//!
//! An EPUB is an untrusted ZIP archive, so every archive-level policy from the
//! EPUB safety plan is enforced here before `rbook` parses any semantics
//! (`DD-008`): member paths become one host-independent canonical key or are
//! rejected, counts and sizes stay inside inclusive boundaries, compression
//! ratios cannot explode, encrypted, symlinked, or unsupported members fail
//! with typed reasons, and control plus chapter resources prove their actual
//! decompressed size instead of trusting archive metadata.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use zip::CompressionMethod;
use zip::read::ZipArchive;

use super::sanitize_path;
use super::structured::{XmlLimits, XmlStructureError, validate_xml_structure};

/// Inclusive archive policy limits; application choices rather than EPUB spec
/// limits. They mirror the initial safety table and only become configurable
/// if real books provide a good reason.
#[derive(Clone, Copy, Debug)]
pub struct ArchiveLimits {
    /// Inclusive maximum compressed size of one source file in bytes.
    pub max_compressed_bytes: u64,
    /// Inclusive maximum number of ZIP members.
    pub max_members: usize,
    /// Inclusive maximum advertised total expansion in bytes.
    pub max_advertised_expansion: u64,
    /// Inclusive maximum decompressed size of container, OPF, NCX, or nav.
    pub max_control_member: u64,
    /// Inclusive maximum decompressed size of one XHTML chapter.
    pub max_chapter_member: u64,
    /// Inclusive maximum per-entry expansion ratio above the small-file
    /// exception.
    pub max_compression_ratio: u64,
    /// Entries whose uncompressed size stays within this bound bypass the
    /// ratio check because their absolute cost is bounded anyway.
    pub small_file_exception: u64,
    /// Pre-parse structure limits for XML control documents and nav candidates.
    pub xml: XmlLimits,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_compressed_bytes: 256 * 1024 * 1024,
            max_members: 10_000,
            max_advertised_expansion: 512 * 1024 * 1024,
            max_control_member: 16 * 1024 * 1024,
            max_chapter_member: 32 * 1024 * 1024,
            max_compression_ratio: 100,
            small_file_exception: 64 * 1024,
            xml: XmlLimits::default(),
        }
    }
}

/// Resource classes with their own decompressed-size boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberClass {
    /// Container, package, NCX, or navigation documents.
    Control,
    /// One XHTML chapter resource.
    Chapter,
    /// Images, fonts, and other lazy resources that text never decodes.
    Other,
}

impl MemberClass {
    /// The inclusive decompressed byte limit for this class.
    ///
    /// Other-class resources share the whole-book expansion budget because no
    /// single lazy member has a smaller dedicated bound yet.
    #[must_use]
    pub const fn limit(self, limits: &ArchiveLimits) -> u64 {
        match self {
            Self::Control => limits.max_control_member,
            Self::Chapter => limits.max_chapter_member,
            Self::Other => limits.max_advertised_expansion,
        }
    }

    /// The human name used in typed diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Chapter => "chapter",
            Self::Other => "resource",
        }
    }
}

/// Classifies one canonical member key for boundary enforcement.
///
/// Control classification covers the exact container name plus OPF and NCX
/// suffixes; XHTML suffixes classify as chapters even before the package
/// declares their role, so no readable resource can dodge the chapter bound.
#[must_use]
pub fn classify_member(key: &str) -> MemberClass {
    let lower = key.to_ascii_lowercase();
    if lower == "meta-inf/container.xml" {
        return MemberClass::Control;
    }
    let extension = Path::new(&lower)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    match extension {
        "opf" | "ncx" => MemberClass::Control,
        "xhtml" | "html" | "htm" => MemberClass::Chapter,
        _ => MemberClass::Other,
    }
}

/// Adds one advertised member size inside an inclusive archive budget.
///
/// `None` represents either arithmetic overflow or a total above the limit;
/// callers must reject both rather than allowing saturation to turn an
/// unrepresentable expansion into an accepted `u64::MAX` total.
#[must_use]
pub fn checked_expansion_total(current: u64, member: u64, limit: u64) -> Option<u64> {
    current.checked_add(member).filter(|total| *total <= limit)
}

/// Rejection reasons for unsafe or ambiguous member names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameRejection {
    /// The name contains interior NUL bytes.
    ContainsNul,
    /// The raw bytes are not UTF-8, so no host-independent key exists.
    NotUtf8,
    /// The name escapes the archive root through a parent segment.
    ParentEscape,
    /// A path segment contains a colon (drive, ADS, or device ambiguity).
    ColonInSegment,
    /// Every segment was empty or dot, leaving no usable name.
    EmptyName,
}

impl NameRejection {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::ContainsNul => "the member name contains NUL bytes",
            Self::NotUtf8 => "the member name is not UTF-8 text",
            Self::ParentEscape => "the member name leaves the archive root",
            Self::ColonInSegment => "a member name segment contains a colon",
            Self::EmptyName => "the member has no usable name",
        }
    }
}

/// Builds one host-independent canonical key for a raw member name.
///
/// Backslashes join as separators so Windows-authored archives have exactly
/// one spelling; dot segments resolve inside the root; parent segments,
/// colons, and NUL bytes reject; trailing dots and spaces strip because
/// extraction-capable hosts would drop them anyway, which keeps duplicate
/// detection sound across hosts.
///
/// # Errors
///
/// Returns the matching [`NameRejection`] for host-ambiguous or unsafe
/// spellings; every accepted name yields exactly one canonical key.
pub fn canonical_key(name: &str) -> Result<String, NameRejection> {
    if name.contains('\0') {
        return Err(NameRejection::ContainsNul);
    }
    if name.starts_with('/') || name.starts_with('\\') {
        // Absolute and UNC-style names are host-bound; one typed rejection
        // covers both spellings before any segment is even considered.
        return Err(NameRejection::ParentEscape);
    }
    let mut segments = Vec::new();
    for segment in name.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => return Err(NameRejection::ParentEscape),
            _ => {
                if segment.contains(':') {
                    return Err(NameRejection::ColonInSegment);
                }
                let trimmed = segment.trim_end_matches(['.', ' ']);
                if !trimmed.is_empty() {
                    segments.push(trimmed.to_owned());
                }
            }
        }
    }
    if segments.is_empty() {
        return Err(NameRejection::EmptyName);
    }
    Ok(segments.join("/"))
}

/// Typed archive-policy failures; each names its book, the reason, and one
/// recovery action without leaking terminal-unsafe bytes.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// The compressed file exceeds [`ArchiveLimits::max_compressed_bytes`].
    #[error(
        "book is too large: '{path}' is {size} bytes and the limit is {limit} bytes; \
         split the book or choose a smaller file"
    )]
    TooLarge {
        /// Safe display path of the rejected file.
        path: String,
        /// Actual byte length observed before any full read.
        size: u64,
        /// Configured inclusive byte limit.
        limit: u64,
    },

    /// An operating-system failure occurred while reading bounded bytes.
    #[error("could not read '{path}': {source}")]
    Read {
        /// Safe display path of the rejected file.
        path: String,
        /// Underlying operating system error.
        #[source]
        source: std::io::Error,
    },

    /// The archive structure itself is unreadable, truncated, or corrupt.
    #[error(
        "could not open '{path}' as an EPUB archive: {detail}; the file may be damaged \
         or not an EPUB book"
    )]
    Malformed {
        /// Safe display path of the rejected file.
        path: String,
        /// Short structural explanation without raw dumps.
        detail: String,
    },

    /// The archive holds more than [`ArchiveLimits::max_members`] entries.
    #[error(
        "book contains too many resources: '{path}' holds {count} entries and the limit \
         is {limit}; the file may be hostile or corrupt"
    )]
    TooManyMembers {
        /// Safe display path of the rejected file.
        path: String,
        /// Observed member count at rejection time.
        count: usize,
        /// Configured inclusive member limit.
        limit: usize,
    },

    /// Advertised expansion crossed [`ArchiveLimits::max_advertised_expansion`].
    #[error(
        "book expands beyond the reading budget: '{member}' helps advertise {total} \
         bytes inside '{path}' and the limit is {limit} bytes; choose a smaller book"
    )]
    AdvertisedExpansionTooLarge {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized canonical key crossing the boundary.
        member: String,
        /// Total advertised expansion at rejection time.
        total: u64,
        /// Configured inclusive byte limit.
        limit: u64,
    },

    /// A member name is unsafe or ambiguous under the canonical-key policy.
    #[error(
        "book resource '{member}' in '{path}' is not acceptable: {reason}; the archive \
         may be hostile or damaged"
    )]
    UnsafeMemberName {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized raw name as stored.
        member: String,
        /// Specific policy reason.
        reason: String,
    },

    /// Two distinct member names collapse onto one canonical key.
    #[error(
        "book resource names collide after normalization on '{member}' in '{path}'; the \
         archive may be hostile or damaged"
    )]
    AmbiguousMemberName {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized canonical key seen twice.
        member: String,
    },

    /// A member carries a compression method outside stored/deflate policy.
    #[error(
        "book resource '{member}' in '{path}' uses unsupported compression ({method}); \
         re-publish the book with standard deflate or stored entries"
    )]
    UnsupportedCompression {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
        /// Compression method description.
        method: String,
    },

    /// Encrypted content appeared before any resource decoding.
    #[error(
        "book resource '{member}' in '{path}' is encrypted; TermLeaf does not open \
         encrypted books"
    )]
    EncryptedMember {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
    },

    /// A member presents itself as a symbolic link instead of content.
    #[error(
        "book resource '{member}' in '{path}' is a symbolic link; the archive may be \
         hostile and was not opened"
    )]
    SymlinkMember {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
    },

    /// Declared sizes contradict what the entry can legitimately contain.
    #[error(
        "book resource '{member}' in '{path}' reports inconsistent sizes; the archive \
         may be hostile or damaged"
    )]
    DishonestMetadata {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
    },

    /// A small compressed member expands past the compression-ratio policy.
    #[error(
        "book resource '{member}' in '{path}' expands to {size} bytes from its \
         compressed form, past the {limit}:1 budget; the entry may be hostile"
    )]
    ExpansionRatioExceeded {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
        /// Declared uncompressed size at rejection time.
        size: u64,
        /// Configured inclusive ratio limit.
        limit: u64,
    },

    /// A member expands past its class boundary by declared or actual size.
    #[error(
        "{kind} resource '{member}' in '{path}' is {size} bytes and the limit is {limit} \
         bytes; the entry may be hostile or the book too large"
    )]
    MemberTooLarge {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name.
        member: String,
        /// Human class name such as "control" or "chapter".
        kind: String,
        /// Declared or counted byte size at rejection time.
        size: u64,
        /// Configured inclusive class limit.
        limit: u64,
    },

    /// Two members claim overlapping compressed data regions.
    #[error(
        "book resources overlap inside '{path}' near '{member}'; the archive may be \
         hostile and was not opened"
    )]
    OverlappingMembers {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized member name of the second overlapping entry.
        member: String,
    },

    /// An XML control document failed its pre-parse structural gate.
    #[error(
        "structured document '{member}' in '{path}' is not safe to parse: {source}; the \
         book may be hostile or damaged"
    )]
    UnsafeXmlStructure {
        /// Safe display path of the rejected file.
        path: String,
        /// Sanitized canonical member key.
        member: String,
        /// Exact structural policy rejection.
        #[source]
        source: XmlStructureError,
    },
}

fn method_name(method: CompressionMethod) -> String {
    match method {
        CompressionMethod::Stored => "stored".to_owned(),
        CompressionMethod::Deflated => "deflate".to_owned(),
        other => format!("{other:?}"),
    }
}

/// One validated archive member as seen during preflight.
#[derive(Clone, Debug)]
pub struct MemberInfo {
    index: usize,
    key: String,
    class: MemberClass,
    declared_size: u64,
}

impl MemberInfo {
    /// Canonical archive key for this member.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Boundary class assigned during preflight.
    #[must_use]
    pub const fn class(&self) -> MemberClass {
        self.class
    }

    /// Declared uncompressed size from the central directory.
    #[must_use]
    pub const fn declared_size(&self) -> u64 {
        self.declared_size
    }
}

/// An archive snapshot that passed every preflight boundary.
///
/// The inspected bytes stay immutable in memory, so semantic parsing sees
/// exactly the content the checks approved; readers never re-open the file
/// from disk.
#[derive(Clone, Debug)]
pub struct PreflightedArchive {
    bytes: Arc<Vec<u8>>,
    display_path: String,
    limits: ArchiveLimits,
    members: Vec<MemberInfo>,
    by_key: BTreeMap<String, usize>,
}

/// Clonable handle over one immutable inspected buffer.
///
/// Semantic parsers such as `rbook` receive their own [`Read`] + [`Seek`]
/// cursor over the exact bytes preflight approved, so no code path can
/// re-open or observe a different file underneath the checks.
#[derive(Clone, Debug)]
pub struct SharedBookBytes(Arc<Vec<u8>>);

impl SharedBookBytes {
    /// A fresh positioned cursor over the shared bytes.
    #[must_use]
    pub fn cursor(&self) -> SharedBookCursor {
        SharedBookCursor {
            bytes: self.clone(),
            position: 0,
        }
    }
}

/// Positioned reader over [`SharedBookBytes`].
#[derive(Debug)]
pub struct SharedBookCursor {
    bytes: SharedBookBytes,
    position: u64,
}

impl Read for SharedBookCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.bytes.0.as_slice();
        #[allow(clippy::cast_possible_truncation)]
        let start = usize::try_from(self.position.min(data.len() as u64)).unwrap_or(usize::MAX);
        let end = start.saturating_add(buf.len()).min(data.len());
        buf[..end - start].copy_from_slice(&data[start..end]);
        let count = end - start;
        self.position += u64::try_from(count).unwrap_or(u64::MAX);
        Ok(count)
    }
}

impl Seek for SharedBookCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let length = self.bytes.0.len() as u64;
        let target = match position {
            SeekFrom::Start(offset) => Some(i128::from(offset)),
            SeekFrom::End(offset) => i128::from(length).checked_add(i128::from(offset)),
            SeekFrom::Current(offset) => i128::from(self.position).checked_add(i128::from(offset)),
        };
        let clamped = target.unwrap_or(0).clamp(0, i128::from(length));
        self.position = u64::try_from(clamped).unwrap_or(u64::MAX);
        Ok(self.position)
    }
}

impl PreflightedArchive {
    /// Validates raw archive bytes against the supplied limits.
    ///
    /// # Errors
    ///
    /// Returns the matching [`ArchiveError`] variant for the first policy
    /// violation found, in structural order: size, parse, count, member
    /// names, methods, encryption, links, ratios, expansion, overlaps, then
    /// actual-byte verification for control and chapter resources.
    pub fn open(
        bytes: Vec<u8>,
        display_path: &str,
        limits: &ArchiveLimits,
    ) -> Result<Self, ArchiveError> {
        if bytes.len() as u64 > limits.max_compressed_bytes {
            return Err(ArchiveError::TooLarge {
                path: display_path.to_owned(),
                size: bytes.len() as u64,
                limit: limits.max_compressed_bytes,
            });
        }
        let (members, by_key) = scan_members(&bytes, display_path, limits)?;
        verify_actual_sizes(&bytes, display_path, limits, &members)?;
        verify_xml_structures(&bytes, display_path, limits, &members)?;

        Ok(Self {
            bytes: Arc::new(bytes),
            display_path: display_path.to_owned(),
            limits: *limits,
            members,
            by_key,
        })
    }

    /// Looks up a validated member by canonical key.
    #[must_use]
    pub fn member(&self, key: &str) -> Option<&MemberInfo> {
        self.by_key.get(key).map(|index| &self.members[*index])
    }

    /// All validated members in archive order.
    #[must_use]
    pub fn members(&self) -> &[MemberInfo] {
        &self.members
    }

    /// Reads and bounds one member by class at access time.
    ///
    /// Actual decompressed bytes are counted again on every call so no caller
    /// can observe more than the class limit allows.
    ///
    /// # Errors
    ///
    /// Returns [`ArchiveError::MemberTooLarge`] above the class limit and
    /// [`ArchiveError::Malformed`] when the entry disappears or stops
    /// matching the preflight pass.
    pub fn read_member(&self, key: &str) -> Result<Vec<u8>, ArchiveError> {
        let limit = self
            .member(key)
            .map_or(self.limits.max_advertised_expansion, |info| {
                info.class.limit(&self.limits)
            });
        self.read_member_bounded(key, limit)
    }

    /// Reads one member without allowing it to exceed the caller's tighter
    /// resource limit or its preflight class limit.
    ///
    /// # Errors
    ///
    /// Returns the same typed failures as [`Self::read_member`].
    pub fn read_member_bounded(
        &self,
        key: &str,
        caller_limit: u64,
    ) -> Result<Vec<u8>, ArchiveError> {
        let info = self.member(key).ok_or_else(|| ArchiveError::Malformed {
            path: self.display_path.clone(),
            detail: format!("resource '{}' vanished after preflight", sanitize_path(key)),
        })?;
        let limit = info.class.limit(&self.limits).min(caller_limit);
        let mut archive = open_archive(&self.bytes, &self.display_path)?;
        let mut entry = archive
            .by_index(info.index)
            .map_err(|error| ArchiveError::Malformed {
                path: self.display_path.clone(),
                detail: malformed_detail(error),
            })?;
        let mut out = Vec::new();
        (&mut entry)
            .take(limit + 1)
            .read_to_end(&mut out)
            .map_err(|error| ArchiveError::Malformed {
                path: self.display_path.clone(),
                detail: format!("could not decompress '{}': {error}", sanitize_path(key)),
            })?;
        if out.len() as u64 > limit {
            return Err(ArchiveError::MemberTooLarge {
                path: self.display_path.clone(),
                member: sanitize_path(key),
                kind: info.class.name().to_owned(),
                size: out.len() as u64,
                limit,
            });
        }
        Ok(out)
    }

    /// A clonable handle over the inspected bytes for semantic parsers.
    #[must_use]
    pub fn shared_bytes(&self) -> SharedBookBytes {
        SharedBookBytes(Arc::clone(&self.bytes))
    }
}

/// Validates one raw member name and reduces it to its canonical key.
///
/// The error pair carries the rejection reason plus a terminal-safe rendering
/// of the raw name for diagnostics.
fn validate_name(raw_name: &[u8]) -> Result<String, (NameRejection, String)> {
    let member = sanitize_path(&String::from_utf8_lossy(raw_name));
    let name = String::from_utf8(raw_name.to_vec())
        .map_err(|_| (NameRejection::NotUtf8, member.clone()))?;
    canonical_key(&name).map_err(|rejection| (rejection, member))
}

/// Validates name, encryption, compression method, and link status for one
/// raw central-directory entry, returning its canonical key.
fn validate_identity(
    entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>,
    display_path: &str,
) -> Result<String, ArchiveError> {
    let raw_name = entry.name_raw().to_vec();
    let key =
        validate_name(&raw_name).map_err(|(rejection, member)| ArchiveError::UnsafeMemberName {
            path: display_path.to_owned(),
            member,
            reason: rejection.reason().to_owned(),
        })?;

    if entry.encrypted() {
        return Err(ArchiveError::EncryptedMember {
            path: display_path.to_owned(),
            member: sanitize_path(&key),
        });
    }
    match entry.compression() {
        CompressionMethod::Stored | CompressionMethod::Deflated => {}
        other => {
            return Err(ArchiveError::UnsupportedCompression {
                path: display_path.to_owned(),
                member: sanitize_path(&key),
                method: method_name(other),
            });
        }
    }
    if entry.is_symlink() {
        return Err(ArchiveError::SymlinkMember {
            path: display_path.to_owned(),
            member: sanitize_path(&key),
        });
    }
    Ok(key)
}

/// Walks the central directory once, enforcing every metadata-level policy.
fn scan_members(
    bytes: &[u8],
    display_path: &str,
    limits: &ArchiveLimits,
) -> Result<(Vec<MemberInfo>, BTreeMap<String, usize>), ArchiveError> {
    let mut members: Vec<MemberInfo> = Vec::new();
    let mut by_key: BTreeMap<String, usize> = BTreeMap::new();
    let mut data_regions: Vec<(u64, u64)> = Vec::new();
    let mut advertised_total = 0u64;
    let mut archive = open_archive(bytes, display_path)?;
    if archive.len() > limits.max_members {
        return Err(ArchiveError::TooManyMembers {
            path: display_path.to_owned(),
            count: archive.len(),
            limit: limits.max_members,
        });
    }

    for index in 0..archive.len() {
        // Metadata scan borrows the raw entry; nothing decompresses here,
        // and encryption stays our own policy decision.
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| ArchiveError::Malformed {
                path: display_path.to_owned(),
                detail: malformed_detail(error),
            })?;
        let key = validate_identity(&entry, display_path)?;

        let (declared, compressed) = (entry.size(), entry.compressed_size());
        if declared == 0 {
            // Zero-byte entries cannot explode; nothing more to bound.
        } else if compressed == 0 {
            // Deflate output is never empty for non-empty input, so a zero
            // compressed size next to content is a lying header.
            return Err(ArchiveError::DishonestMetadata {
                path: display_path.to_owned(),
                member: sanitize_path(&key),
            });
        } else if declared > limits.small_file_exception {
            let allowed = compressed.saturating_mul(limits.max_compression_ratio);
            if declared > allowed {
                return Err(ArchiveError::ExpansionRatioExceeded {
                    path: display_path.to_owned(),
                    member: sanitize_path(&key),
                    size: declared,
                    limit: limits.max_compression_ratio,
                });
            }
        }

        if let Some(start) = entry.data_start() {
            let end = start.saturating_add(compressed);
            if data_regions
                .iter()
                .any(|(seen_start, seen_end)| start < *seen_end && *seen_start < end)
            {
                return Err(ArchiveError::OverlappingMembers {
                    path: display_path.to_owned(),
                    member: sanitize_path(&key),
                });
            }
            data_regions.push((start, end));
        }

        let class = classify_member(&key);
        if class != MemberClass::Other && declared > class.limit(limits) {
            return Err(ArchiveError::MemberTooLarge {
                path: display_path.to_owned(),
                member: sanitize_path(&key),
                kind: class.name().to_owned(),
                size: declared,
                limit: class.limit(limits),
            });
        }

        let Some(next_total) =
            checked_expansion_total(advertised_total, declared, limits.max_advertised_expansion)
        else {
            return Err(ArchiveError::AdvertisedExpansionTooLarge {
                path: display_path.to_owned(),
                member: sanitize_path(&key),
                total: advertised_total.saturating_add(declared),
                limit: limits.max_advertised_expansion,
            });
        };
        advertised_total = next_total;

        if by_key.contains_key(&key) {
            return Err(ArchiveError::AmbiguousMemberName {
                path: display_path.to_owned(),
                member: sanitize_path(&key),
            });
        }
        by_key.insert(key.clone(), members.len());
        members.push(MemberInfo {
            index,
            key,
            class,
            declared_size: declared,
        });
    }
    Ok((members, by_key))
}

fn open_archive<'a>(
    bytes: &'a [u8],
    display_path: &str,
) -> Result<ZipArchive<Cursor<&'a [u8]>>, ArchiveError> {
    ZipArchive::new(Cursor::new(bytes)).map_err(|error| ArchiveError::Malformed {
        path: display_path.to_owned(),
        detail: malformed_detail(error),
    })
}

fn malformed_detail(error: zip::result::ZipError) -> String {
    match error {
        zip::result::ZipError::InvalidArchive(detail) => {
            format!("the archive structure is invalid: {detail}")
        }
        zip::result::ZipError::FileNotFound => "an expected entry is missing".to_owned(),
        other => format!("the archive could not be parsed: {other}"),
    }
}

/// Counts actual decompressed bytes for every control and chapter member.
///
/// Text extraction touches exactly these classes, so each must decompress to
/// its declared size inside the class bound before any XML parser allocates;
/// images and fonts stay lazy and unverified.
fn verify_actual_sizes(
    bytes: &[u8],
    display_path: &str,
    limits: &ArchiveLimits,
    members: &[MemberInfo],
) -> Result<(), ArchiveError> {
    let mut archive = open_archive(bytes, display_path)?;
    for info in members {
        if info.class == MemberClass::Other || info.declared_size == 0 {
            continue;
        }
        let limit = info.class.limit(limits);
        let mut entry = archive
            .by_index(info.index)
            .map_err(|error| ArchiveError::Malformed {
                path: display_path.to_owned(),
                detail: malformed_detail(error),
            })?;
        let mut counted = 0u64;
        let mut chunk = [0u8; 16 * 1024];
        loop {
            let read = entry
                .read(&mut chunk)
                .map_err(|error| ArchiveError::Malformed {
                    path: display_path.to_owned(),
                    detail: format!(
                        "could not decompress '{}': {error}",
                        sanitize_path(&info.key)
                    ),
                })?;
            if read == 0 {
                break;
            }
            counted += read as u64;
            if counted > limit {
                return Err(ArchiveError::MemberTooLarge {
                    path: display_path.to_owned(),
                    member: sanitize_path(&info.key),
                    kind: info.class.name().to_owned(),
                    size: counted,
                    limit,
                });
            }
        }
        if counted != info.declared_size {
            return Err(ArchiveError::DishonestMetadata {
                path: display_path.to_owned(),
                member: sanitize_path(&info.key),
            });
        }
    }
    Ok(())
}

/// Gates every known control document and every HTML nav candidate before the
/// package parser can inspect semantics. EPUB 3 nav paths are arbitrary, so
/// all XHTML/HTML/HTM members take the same cheap structural scan.
fn verify_xml_structures(
    bytes: &[u8],
    display_path: &str,
    limits: &ArchiveLimits,
    members: &[MemberInfo],
) -> Result<(), ArchiveError> {
    let mut archive = open_archive(bytes, display_path)?;
    for info in members {
        if info.class == MemberClass::Other {
            continue;
        }

        let mut entry = archive
            .by_index(info.index)
            .map_err(|error| ArchiveError::Malformed {
                path: display_path.to_owned(),
                detail: malformed_detail(error),
            })?;
        let mut source = Vec::with_capacity(usize::try_from(info.declared_size).unwrap_or(0));
        entry
            .read_to_end(&mut source)
            .map_err(|error| ArchiveError::Malformed {
                path: display_path.to_owned(),
                detail: format!(
                    "could not decompress '{}': {error}",
                    sanitize_path(&info.key)
                ),
            })?;
        validate_xml_structure(&source, limits.xml).map_err(|source| {
            ArchiveError::UnsafeXmlStructure {
                path: display_path.to_owned(),
                member: sanitize_path(&info.key),
                source,
            }
        })?;
    }
    Ok(())
}

/// Reads a local book file once and runs the full preflight pipeline.
///
/// The compressed-size boundary applies to metadata before allocation and to
/// the guarded read, so neither a lying header nor a racing writer can force
/// an unbounded buffer.
///
/// # Errors
///
/// Returns [`ArchiveError::TooLarge`] above the compressed limit,
/// [`ArchiveError::Read`] for operating-system failures, and every other
/// [`ArchiveError`] variant from [`PreflightedArchive::open`].
pub fn open_book_archive(
    path: &Path,
    limits: &ArchiveLimits,
) -> Result<PreflightedArchive, ArchiveError> {
    let display = sanitize_path(&path.display().to_string());
    let mut file = File::open(path).map_err(|source| ArchiveError::Read {
        path: display.clone(),
        source,
    })?;

    let declared = file.metadata().map_err(|source| ArchiveError::Read {
        path: display.clone(),
        source,
    })?;
    if declared.len() > limits.max_compressed_bytes {
        return Err(ArchiveError::TooLarge {
            path: display,
            size: declared.len(),
            limit: limits.max_compressed_bytes,
        });
    }

    let mut bytes = Vec::new();
    (&mut file)
        .take(limits.max_compressed_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| ArchiveError::Read {
            path: display.clone(),
            source,
        })?;
    if bytes.len() as u64 > limits.max_compressed_bytes {
        return Err(ArchiveError::TooLarge {
            path: display,
            size: bytes.len() as u64,
            limit: limits.max_compressed_bytes,
        });
    }

    PreflightedArchive::open(bytes, &display, limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "book.epub";

    fn tiny_limits() -> ArchiveLimits {
        ArchiveLimits {
            max_compressed_bytes: 4096,
            max_members: 16,
            max_advertised_expansion: 1024,
            max_control_member: 64,
            max_chapter_member: 96,
            max_compression_ratio: 100,
            small_file_exception: 32,
            xml: XmlLimits::default(),
        }
    }

    /// Builds a stored-entry archive with full byte-level control so tests
    /// can craft lying headers, hostile flags, and overlapping entries.
    struct HandEntry {
        name: String,
        data: Vec<u8>,
        method: u16,
        flags: u16,
        crc_override: Option<u32>,
        declared_size: Option<u32>,
        declared_compressed: Option<u32>,
    }

    impl HandEntry {
        fn stored(name: &str, data: &[u8]) -> Self {
            Self {
                name: name.to_owned(),
                data: data.to_vec(),
                method: 0,
                flags: 0,
                crc_override: None,
                declared_size: None,
                declared_compressed: None,
            }
        }
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 != 0 {
                    0xEDB8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *slot = value;
        }
        let mut crc = 0xFFFF_FFFFu32;
        for byte in data {
            crc = table[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8);
        }
        !crc
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "test fixtures stay far below every field width"
    )]
    fn build_archive(entries: &[HandEntry]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for entry in entries {
            let crc = entry.crc_override.unwrap_or_else(|| crc32(&entry.data));
            let size = entry.declared_size.unwrap_or(entry.data.len() as u32);
            let compressed = entry.declared_compressed.unwrap_or(entry.data.len() as u32);
            let header_offset = out.len();
            out.extend_from_slice(&0x0403_4B50_u32.to_le_bytes());
            out.extend_from_slice(&20_u16.to_le_bytes());
            out.extend_from_slice(&entry.flags.to_le_bytes());
            out.extend_from_slice(&entry.method.to_le_bytes());
            out.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&compressed.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0_u16.to_le_bytes());
            out.extend_from_slice(entry.name.as_bytes());
            out.extend_from_slice(&entry.data);

            directory.extend_from_slice(&0x0201_4B50_u32.to_le_bytes());
            directory.extend_from_slice(&20_u16.to_le_bytes());
            directory.extend_from_slice(&20_u16.to_le_bytes());
            directory.extend_from_slice(&entry.flags.to_le_bytes());
            directory.extend_from_slice(&entry.method.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&crc.to_le_bytes());
            directory.extend_from_slice(&compressed.to_le_bytes());
            directory.extend_from_slice(&size.to_le_bytes());
            directory.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u16.to_le_bytes());
            directory.extend_from_slice(&0_u32.to_le_bytes());
            directory.extend_from_slice(&(header_offset as u32).to_le_bytes());
            directory.extend_from_slice(entry.name.as_bytes());
        }
        let directory_offset = out.len();
        out.extend_from_slice(&directory);
        out.extend_from_slice(&0x0605_4B50_u32.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out
    }

    fn open(entries: &[HandEntry], limits: &ArchiveLimits) -> Result<(), ArchiveError> {
        PreflightedArchive::open(build_archive(entries), PATH, limits).map(|_| ())
    }

    /// Overwrites one little-endian `u32` inside the named central-directory
    /// header. Field offsets follow the fixed 46-byte central header layout:
    /// 20 = compressed size, 24 = uncompressed size, 42 = local offset.
    fn patch_central_field(bytes: &mut [u8], name: &str, field: usize, value: u32) {
        let signature = 0x0201_4B50_u32.to_le_bytes();
        let mut start = 0;
        while let Some(found) = bytes[start..]
            .windows(4)
            .position(|window| window == signature)
        {
            let header = start + found;
            let name_length = u16::from_le_bytes([bytes[header + 28], bytes[header + 29]]) as usize;
            let stored_name = &bytes[header + 46..header + 46 + name_length];
            if stored_name == name.as_bytes() {
                let at = header + field;
                bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
                return;
            }
            start = header + 4;
        }
        panic!("central header for {name} not found");
    }

    #[test]
    fn sec_001_unsafe_or_ambiguous_member_names_reject_with_typed_reasons() {
        let cases = [
            ("/etc/passwd", "leaves the archive root"),
            ("../escape.xhtml", "leaves the archive root"),
            ("a/../../b", "leaves the archive root"),
            ("bad\0name", "contains NUL bytes"),
            ("device:stream", "segment contains a colon"),
            ("", "no usable name"),
        ];
        for (name, expected_reason) in cases {
            let error = open(&[HandEntry::stored(name, b"x")], &tiny_limits())
                .expect_err("unsafe names are rejected");
            let ArchiveError::UnsafeMemberName { reason, .. } = error else {
                panic!("{name}: expected UnsafeMemberName, got {error:?}");
            };
            assert!(
                reason.contains(expected_reason),
                "{name}: {reason} does not mention {expected_reason}"
            );
        }

        // Two spellings collapsing onto one canonical key are ambiguous.
        let ambiguous = open(
            &[
                HandEntry::stored("a/b.xhtml", b"one"),
                HandEntry::stored("a\\b.xhtml", b"two"),
            ],
            &tiny_limits(),
        )
        .expect_err("colliding keys reject");
        assert!(matches!(
            ambiguous,
            ArchiveError::AmbiguousMemberName { .. }
        ));
    }

    #[test]
    fn sec_002_symlink_encrypted_and_unsupported_entries_match_policy_errors() {
        // Symbolic link: unix mode bits mark the entry through the writer.
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o120_777);
            writer
                .start_file("OEBPS/link.xhtml", options)
                .expect("write");
            std::io::Write::write_all(&mut writer, b"target").expect("body");
            writer.finish().expect("finish");
        }
        let mut bytes = buffer.into_inner();
        let position = bytes
            .windows(4)
            .position(|window| window == 0x0201_4B50_u32.to_le_bytes())
            .expect("central signature present");
        // External attributes sit 38 bytes into a central header and store
        // the Unix mode in their high 16 bits; the writer normalizes modes
        // back to regular files, so restore the link bits there.
        let attribute_offset = position + 38;
        bytes[attribute_offset..attribute_offset + 4]
            .copy_from_slice(&(0o120_777_u32 << 16).to_le_bytes());
        let error = PreflightedArchive::open(bytes, PATH, &tiny_limits())
            .expect_err("symlinks are rejected");
        assert!(
            matches!(error, ArchiveError::SymlinkMember { .. }),
            "{error:?}"
        );

        // Encrypted flag on both headers must produce the typed message.
        let mut encrypted = HandEntry::stored("OEBPS/c.xhtml", b"secret");
        encrypted.flags = 1;
        let error = open(&[encrypted], &tiny_limits()).expect_err("encryption rejects");
        let ArchiveError::EncryptedMember { member, .. } = error else {
            panic!("expected EncryptedMember, got {error:?}");
        };
        assert!(member.contains("c.xhtml"), "{member}");

        // Unsupported compression methods never reach a decoder.
        let mut exotic = HandEntry::stored("OEBPS/c.xhtml", b"data");
        exotic.method = 98;
        let error = open(&[exotic], &tiny_limits()).expect_err("method policy rejects");
        let ArchiveError::UnsupportedCompression { method, .. } = error else {
            panic!("expected UnsupportedCompression, got {error:?}");
        };
        assert!(!method.is_empty(), "{method}");
    }

    #[test]
    fn sec_003_compressed_file_boundary_is_inclusive() {
        let bytes = build_archive(&[HandEntry::stored("mimetype", b"application/epub+zip")]);
        let limits = ArchiveLimits {
            max_compressed_bytes: bytes.len() as u64,
            ..tiny_limits()
        };
        assert!(
            PreflightedArchive::open(bytes.clone(), PATH, &limits).is_ok(),
            "the exact boundary passes"
        );
        let tight = ArchiveLimits {
            max_compressed_bytes: bytes.len() as u64 - 1,
            ..tiny_limits()
        };
        let error = PreflightedArchive::open(bytes, PATH, &tight).expect_err("over-limit rejects");
        assert!(matches!(error, ArchiveError::TooLarge { .. }), "{error:?}");
    }

    #[test]
    fn sec_004_member_count_boundary_is_inclusive() {
        let make = |count| {
            let entries: Vec<_> = (0..count)
                .map(|index| HandEntry::stored(&format!("m{index}"), b"x"))
                .collect();
            build_archive(&entries)
        };
        let limits = ArchiveLimits {
            max_members: 4,
            ..tiny_limits()
        };
        assert!(PreflightedArchive::open(make(4), PATH, &limits).is_ok());
        let error =
            PreflightedArchive::open(make(5), PATH, &limits).expect_err("too many members reject");
        let ArchiveError::TooManyMembers { count, limit, .. } = error else {
            panic!("expected TooManyMembers, got {error:?}");
        };
        assert_eq!((count, limit), (5, 4));
    }

    #[test]
    fn sec_005_advertised_expansion_boundary_is_inclusive_per_entry_and_aggregate() {
        // Aggregate: two honest members whose sum crosses the injected limit.
        let limits = ArchiveLimits {
            max_advertised_expansion: 10,
            ..tiny_limits()
        };
        let entries = [
            HandEntry::stored("a.txt", &[0u8; 6]),
            HandEntry::stored("b.txt", &[0u8; 6]),
        ];
        let error = open(&entries, &limits).expect_err("aggregate expansion rejects");
        let ArchiveError::AdvertisedExpansionTooLarge { total, limit, .. } = error else {
            panic!("expected AdvertisedExpansionTooLarge, got {error:?}");
        };
        assert_eq!((total, limit), (12, 10));
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            writer
                .start_file("OEBPS/bomb.xhtml", zip::write::SimpleFileOptions::default())
                .expect("write");
            std::io::Write::write_all(&mut writer, &[7u8; 200]).expect("body");
            writer.finish().expect("finish");
        }
        let mut bytes = buffer.into_inner();
        patch_central_field(&mut bytes, "OEBPS/bomb.xhtml", 24, 1000);
        let roomy = ArchiveLimits {
            max_advertised_expansion: u64::MAX,
            max_chapter_member: u64::from(u32::MAX),
            max_compression_ratio: u64::MAX,
            ..tiny_limits()
        };
        let error =
            PreflightedArchive::open(bytes, PATH, &roomy).expect_err("dishonest sizes reject");
        assert!(
            matches!(error, ArchiveError::DishonestMetadata { .. }),
            "unexpected variant: {error:?}"
        );
    }

    #[test]
    fn sec_006_control_members_fail_before_semantic_parsing_at_the_boundary() {
        let limits = ArchiveLimits {
            max_control_member: 8,
            ..tiny_limits()
        };
        let exact = [HandEntry::stored("OEBPS/content.opf", &[0u8; 8])];
        assert!(
            open(&exact, &limits).is_ok(),
            "exact control boundary passes"
        );

        let over = [HandEntry::stored("OEBPS/content.opf", &[0u8; 9])];
        let error = open(&over, &limits).expect_err("declared control size rejects");
        let ArchiveError::MemberTooLarge {
            kind, size, limit, ..
        } = error
        else {
            panic!("expected MemberTooLarge, got {error:?}");
        };
        assert_eq!((kind.as_str(), size, limit), ("control", 9, 8));

        let nav_over = [HandEntry::stored("META-INF/container.xml", &[0u8; 12])];
        let error = open(&nav_over, &limits).expect_err("container bound applies");
        assert!(matches!(error, ArchiveError::MemberTooLarge { .. }));

        let ncx_over = [HandEntry::stored("OEBPS/toc.ncx", &[0u8; 12])];
        let error = open(&ncx_over, &limits).expect_err("NCX bound applies");
        assert!(matches!(error, ArchiveError::MemberTooLarge { .. }));
    }

    #[test]
    fn sec_007_chapter_boundary_yields_one_exact_typed_result() {
        let limits = ArchiveLimits {
            max_chapter_member: 16,
            ..tiny_limits()
        };
        let entries = [
            HandEntry::stored("mimetype", b"application/epub+zip"),
            HandEntry::stored("OEBPS/chapter.xhtml", &[0u8; 17]),
        ];
        let error = open(&entries, &limits).expect_err("chapter bound rejects");
        let ArchiveError::MemberTooLarge {
            kind, size, limit, ..
        } = error
        else {
            panic!("expected MemberTooLarge, got {error:?}");
        };
        assert_eq!((kind.as_str(), size, limit), ("chapter", 17, 16));
    }

    #[test]
    fn sec_008_ratio_formula_zero_byte_and_small_file_rules_hold_without_overflow() {
        let limits = ArchiveLimits {
            small_file_exception: 32,
            max_compression_ratio: 100,
            ..tiny_limits()
        };

        // Zero-byte stored entries pass with zero compressed size.
        let empty = [HandEntry::stored("empty.txt", b"")];
        assert!(open(&empty, &limits).is_ok());

        // Small files bypass the ratio check entirely.
        let mut small = HandEntry::stored("small.txt", &[7u8; 8]);
        small.declared_size = None;
        small.declared_compressed = Some(1);
        assert!(open(std::slice::from_ref(&small), &limits).is_ok());

        // Above the exception, the ratio formula is inclusive at the edge.
        let mut edge = HandEntry::stored("edge.txt", &[7u8; 33]);
        edge.declared_compressed = Some(1); // allowed = 1 * 100 >= 33? no: 100 > 33 passes
        assert!(open(std::slice::from_ref(&edge), &limits).is_ok());

        let mut over = HandEntry::stored("over.txt", &[7u8; 202]);
        over.declared_compressed = Some(2); // allowed = 200 < 202
        let error = open(&[over], &limits).expect_err("ratio breach rejects");
        let ArchiveError::ExpansionRatioExceeded { size, limit, .. } = error else {
            panic!("expected ExpansionRatioExceeded, got {error:?}");
        };
        assert_eq!((size, limit), (202, 100));
    }

    #[test]
    fn sec_010_truncated_corrupt_and_crc_broken_archives_stay_typed() {
        // Truncated before any structure appears.
        let truncated = vec![0x50, 0x4B, 0x03];
        let error = PreflightedArchive::open(truncated, PATH, &tiny_limits())
            .expect_err("truncation rejects");
        assert!(matches!(error, ArchiveError::Malformed { .. }), "{error:?}");

        // Corrupt central directory signature.
        let mut corrupt = build_archive(&[HandEntry::stored("a.txt", b"data")]);
        let position = corrupt
            .windows(4)
            .position(|window| window == 0x0201_4B50_u32.to_le_bytes())
            .expect("central signature present");
        corrupt[position] ^= 0xFF;
        let error = PreflightedArchive::open(corrupt, PATH, &tiny_limits())
            .expect_err("corruption rejects");
        assert!(matches!(error, ArchiveError::Malformed { .. }));

        // CRC mismatch surfaces while counting actual bytes.
        let mut lying = HandEntry::stored("OEBPS/c.xhtml", b"actual data");
        lying.crc_override = Some(0xDEAD_BEEF);
        let error =
            open(std::slice::from_ref(&lying), &tiny_limits()).expect_err("crc mismatch rejects");
        assert!(matches!(error, ArchiveError::Malformed { .. }), "{error:?}");
    }

    #[test]
    fn sec_002_overlapping_data_regions_reject_before_semantic_parsing() {
        // Two central-directory entries pointing at one compressed region is
        // the classic overlap attack; the second claim must reject.
        let entries = [
            HandEntry::stored("OEBPS/a.xhtml", b"first body"),
            HandEntry::stored("OEBPS/b.xhtml", b"second body"),
        ];
        let mut bytes = build_archive(&entries);
        let signature = 0x0403_4B50_u32.to_le_bytes();
        #[allow(clippy::cast_possible_truncation)]
        let first_local_offset = bytes
            .windows(4)
            .position(|window| window == signature)
            .expect("local signature present") as u32;
        // Repoint the second member's central local-header offset (field 42).
        let central_signature = 0x0201_4B50_u32.to_le_bytes();
        let mut found = Vec::new();
        let mut start = 0;
        while let Some(position) = bytes[start..]
            .windows(4)
            .position(|window| window == central_signature)
        {
            found.push(start + position);
            start += position + 4;
        }
        assert_eq!(found.len(), 2, "two central headers exist");
        let second_central = found[1];
        bytes[second_central + 42..second_central + 46]
            .copy_from_slice(&first_local_offset.to_le_bytes());

        let error = PreflightedArchive::open(bytes, PATH, &tiny_limits())
            .expect_err("overlapping members reject");
        assert!(
            matches!(error, ArchiveError::OverlappingMembers { .. }),
            "{error:?}"
        );
    }

    #[test]
    fn sec_011_host_dependent_spellings_reduce_to_one_canonical_key() {
        use NameRejection::{ColonInSegment, ContainsNul, EmptyName, ParentEscape};
        assert_eq!(canonical_key("/abs/path").unwrap_err(), ParentEscape);
        assert_eq!(canonical_key("//server/share/x").unwrap_err(), ParentEscape);
        assert_eq!(canonical_key("./a/./b/").as_deref(), Ok("a/b"));
        assert_eq!(
            canonical_key("dir\\file.txt").as_deref(),
            Ok("dir/file.txt")
        );
        assert_eq!(canonical_key("name. . ").as_deref(), Ok("name"));
        assert_eq!(canonical_key("C:/temp/x").unwrap_err(), ColonInSegment);
        assert_eq!(canonical_key("a\0b").unwrap_err(), ContainsNul);
        assert_eq!(canonical_key(". / . /").unwrap_err(), EmptyName);
        // Case and Unicode spellings each keep exactly one canonical form.
        assert_ne!(canonical_key("A.txt"), canonical_key("a.txt"));

        // Deterministic pseudo-random hostile names never panic and always
        // resolve to either a valid key or one typed rejection.
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..512 {
            let length = next() % 24;
            let name: String = (0..length)
                .map(|_| {
                    let pick = next() % 8;
                    match pick {
                        0 => '/',
                        1 => '\\',
                        2 => '.',
                        3 => ':',
                        4 => '\0',
                        5 => ' ',
                        _ => char::from(b'a' + (next() % 26) as u8),
                    }
                })
                .collect();
            if let Ok(key) = canonical_key(&name) {
                assert!(!key.contains('\0'));
                assert!(!key.split('/').any(|segment| segment == ".."));
                assert!(key.split('/').all(|segment| !segment.contains(':')));
            }
        }
    }
}
