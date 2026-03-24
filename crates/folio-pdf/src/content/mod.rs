//! PDF content stream parsing and graphics state.
//!
//! This module parses page content streams into a sequence of typed operations,
//! maintains the graphics state stack, and provides an element-level iterator.

mod gstate;
mod ops;
mod parser;

pub use gstate::{GraphicsState, GraphicsStateStack};
pub use ops::{ContentOp, PathSegment, TextOp};
pub use parser::parse_content_stream;
