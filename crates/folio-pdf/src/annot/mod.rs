//! PDF annotation types — reading, creating, and modifying annotations.
//!
//! Covers all 28 PDF annotation types defined in ISO 32000-2:2020 §12.5.

mod annot;
mod border;
mod types;

pub use annot::{Annot, AnnotFlags, AnnotType};
pub use border::{BorderStyle, BorderStyleType};
pub use types::*;
