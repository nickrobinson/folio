//! # Folio PDF — A comprehensive PDF library for Rust
//!
//! Folio provides full-featured PDF reading, writing, and manipulation.
//!
//! ## Quick Start
//!
//! ### Open a PDF and read pages
//!
//! ```no_run
//! use folio_pdf::prelude::*;
//!
//! let mut doc = PdfDoc::open("document.pdf")?;
//! println!("Pages: {}", doc.page_count()?);
//!
//! let page = doc.get_page(1)?;
//! println!("Size: {}x{}", page.width(), page.height());
//! # Ok::<(), folio_pdf::Error>(())
//! ```
//!
//! ### Extract text
//!
//! ```no_run
//! use folio_pdf::prelude::*;
//!
//! let mut doc = PdfDoc::open("document.pdf")?;
//! let page = doc.get_page(1)?;
//! let text = TextExtractor::extract_from_page(&page, doc.cos_mut())?;
//! println!("{}", text);
//! # Ok::<(), folio_pdf::Error>(())
//! ```
//!
//! ### Read form fields
//!
//! ```no_run
//! use folio_pdf::prelude::*;
//!
//! let data = std::fs::read("form.pdf")?;
//! let mut doc = CosDoc::open(data)?;
//! for field in AcroForm::get_fields(&mut doc)? {
//!     println!("{}: {:?} = {:?}", field.name(), field.field_type(), field.value());
//! }
//! # Ok::<(), folio_pdf::Error>(())
//! ```
//!
//! ### Read bookmarks
//!
//! ```no_run
//! use folio_pdf::prelude::*;
//!
//! let data = std::fs::read("document.pdf")?;
//! let mut doc = CosDoc::open(data)?;
//! for (bookmark, depth) in Bookmark::get_all(&mut doc)? {
//!     println!("{}{}", "  ".repeat(depth as usize), bookmark.title());
//! }
//! # Ok::<(), folio_pdf::Error>(())
//! ```
//!
//! ## Module Organization
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`core`] | Primitive types: `Rect`, `Matrix2D`, `Point`, `ColorPt`, `PdfDate`, errors |
//! | [`cos`] | Low-level PDF object model, parser, serializer, `CosDoc` |
//! | [`doc`] | High-level `PdfDoc`, `Page`, `DocInfo` |
//! | [`content`] | Content stream parsing, operators, graphics state |
//! | [`font`] | Font loading, encoding tables, Unicode mapping |
//! | [`text`] | Text extraction and search |
//! | [`annot`] | Annotation types (highlight, text, link, widget, etc.) |
//! | [`forms`] | AcroForm fields (text, checkbox, radio, choice, signature) |
//! | [`nav`] | Bookmarks, destinations, actions |
//! | [`filters`] | Stream compression/decompression (Flate, ASCII85, LZW, etc.) |

/// Core types: `Rect`, `Matrix2D`, `Point`, `ColorPt`, `PdfDate`, `FolioError`.
pub mod core;

/// Low-level COS object model and PDF parser.
pub mod cos;

/// High-level PDF document and page model.
pub mod doc;

/// Content stream parsing and graphics state.
pub mod content;

/// Font loading, encoding, and Unicode mapping.
pub mod font;

/// Text extraction.
pub mod text;

/// Image embedding and extraction.
pub mod image;

/// PDF annotations.
pub mod annot;

/// Interactive form fields.
pub mod forms;

/// Bookmarks, destinations, and actions.
pub mod nav;

/// Stream filters (compression/decompression).
pub mod filters;

/// Convenience type alias for the library's error type.
pub type Error = crate::core::FolioError;

/// Convenience type alias for Results.
pub type Result<T> = crate::core::Result<T>;

/// Prelude — import this for the most commonly used types.
///
/// ```
/// use folio_pdf::prelude::*;
/// ```
pub mod prelude {
    // Document types
    pub use crate::doc::{DocInfo, Page, PdfDoc};

    // Low-level access
    pub use crate::cos::CosDoc;

    // Text extraction and search
    pub use crate::text::{SearchOptions, SearchResult, TextExtractor, TextSearch};

    // Images
    pub use crate::image::{PdfImage, add_image_to_page};

    // Annotations
    pub use crate::annot::{Annot, AnnotFlags, AnnotType};

    // Forms
    pub use crate::forms::{AcroForm, Field, FieldFlags, FieldType};

    // Navigation
    pub use crate::nav::{Action, ActionType, Bookmark, Destination, FitType};

    // Core types
    pub use crate::core::{ColorPt, FolioError, Matrix2D, PdfDate, Point, QuadPoint, Rect, Result};
}
