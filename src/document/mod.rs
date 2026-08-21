//! Logical document model shared by every supported book format.
//!
//! The model is independent of terminal width and of any UI crate: a layout
//! pass turns [`Document`] content into visual rows, while reading positions
//! always address the canonical logical text recorded here.

mod error;
pub mod model;
pub mod text;

pub use error::{DocumentError, PositionError};
pub use model::{Block, BlockKind, Document, DocumentId, Position, Section};
pub use text::TextLimits;
