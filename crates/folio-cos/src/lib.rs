//! COS (Carousel Object System) object model and PDF parser.
//!
//! This is the foundation of the Folio PDF library. It provides:
//! - The PDF object model (`PdfObject` enum)
//! - A PDF tokenizer and parser
//! - Cross-reference table parsing (both table and stream formats)
//! - PDF serialization
//! - The `CosDoc` document type for low-level PDF access

mod document;
mod object;
pub mod parser;
mod serialize;
pub mod tokenizer;
mod xref;

pub use document::CosDoc;
pub use object::{ObjectId, PdfObject, PdfStream};
pub use xref::XrefEntry;
