//! Core primitive types for the Folio PDF library.

mod color;
mod date;
mod error;
mod matrix;
mod point;
mod rect;

pub use color::ColorPt;
pub use date::PdfDate;
pub use error::{FolioError, Result};
pub use matrix::Matrix2D;
pub use point::{Point, QuadPoint};
pub use rect::Rect;
