//! High-level PDF document model.
//!
//! Provides `PdfDoc` and `Page` types that wrap the low-level COS layer
//! with PDF-specific semantics.

mod info;
mod page;
mod pdfdoc;

pub use info::DocInfo;
pub use page::Page;
pub use pdfdoc::PdfDoc;
