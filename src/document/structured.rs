//! Pre-parse structural gates for untrusted XML control documents.

/// Inclusive XML structure limits applied before semantic parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XmlLimits {
    /// Maximum nested element depth.
    pub max_depth: usize,
    /// Maximum number of element openings.
    pub max_nodes: usize,
}

impl Default for XmlLimits {
    fn default() -> Self {
        Self {
            max_depth: 256,
            max_nodes: 1_000_000,
        }
    }
}

/// A structural XML policy rejection found before semantic parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum XmlStructureError {
    /// Element nesting crossed the inclusive depth limit.
    #[error("XML depth {depth} exceeds the {limit} depth limit")]
    TooDeep {
        /// Depth observed at rejection.
        depth: usize,
        /// Configured inclusive depth limit.
        limit: usize,
    },
    /// Element openings crossed the inclusive node limit.
    #[error("XML opening count {nodes} exceeds the {limit} node limit")]
    TooManyNodes {
        /// Opening count observed at rejection.
        nodes: usize,
        /// Configured inclusive node limit.
        limit: usize,
    },
    /// A DTD declaration was present.
    #[error("DTD and entity declarations are not allowed")]
    DtdDeclaration,
}

/// Checks XML structure without building a tree or resolving declarations.
///
/// Comments, CDATA, and processing instructions are skipped. Any other
/// `<!...>` construct is a DTD declaration and rejects, including `DOCTYPE`,
/// `ENTITY`, `ELEMENT`, `ATTLIST`, and `NOTATION` forms.
///
/// # Errors
///
/// Returns the first limit or declaration rejection in source order.
pub fn validate_xml_structure(source: &[u8], limits: XmlLimits) -> Result<(), XmlStructureError> {
    let mut cursor = 0;
    let mut depth = 0usize;
    let mut nodes = 0usize;

    while let Some(relative) = source[cursor..].iter().position(|byte| *byte == b'<') {
        let start = cursor + relative;
        let rest = &source[start..];

        if rest.starts_with(b"<!--") {
            cursor = skip_through(source, start + 4, b"-->");
            continue;
        }
        if rest.starts_with(b"<![CDATA[") {
            cursor = skip_through(source, start + 9, b"]]>");
            continue;
        }
        if rest.starts_with(b"<?") {
            cursor = skip_through(source, start + 2, b"?>");
            continue;
        }
        if rest.starts_with(b"<!") {
            return Err(XmlStructureError::DtdDeclaration);
        }
        if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            cursor = tag_end(source, start + 2).0;
            continue;
        }

        nodes = nodes.saturating_add(1);
        if nodes > limits.max_nodes {
            return Err(XmlStructureError::TooManyNodes {
                nodes,
                limit: limits.max_nodes,
            });
        }
        let (end, self_closing) = tag_end(source, start + 1);
        let opening_depth = depth.saturating_add(1);
        if opening_depth > limits.max_depth {
            return Err(XmlStructureError::TooDeep {
                depth: opening_depth,
                limit: limits.max_depth,
            });
        }
        if !self_closing {
            depth = opening_depth;
        }
        cursor = end;
    }
    Ok(())
}

fn skip_through(source: &[u8], start: usize, terminator: &[u8]) -> usize {
    source[start..]
        .windows(terminator.len())
        .position(|window| window == terminator)
        .map_or(source.len(), |relative| start + relative + terminator.len())
}

/// Returns the byte after a tag plus whether its final non-space byte is `/`.
fn tag_end(source: &[u8], start: usize) -> (usize, bool) {
    let mut quote = None;
    let mut cursor = start;
    while cursor < source.len() {
        let byte = source[cursor];
        match (quote, byte) {
            (Some(expected), current) if current == expected => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => {
                let self_closing = source[start..cursor]
                    .iter()
                    .rfind(|byte| !byte.is_ascii_whitespace())
                    .is_some_and(|byte| *byte == b'/');
                return (cursor + 1, self_closing);
            }
            _ => {}
        }
        cursor += 1;
    }
    (source.len(), false)
}
