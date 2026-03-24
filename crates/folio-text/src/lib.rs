//! PDF text extraction and search.
//!
//! Extracts text from PDF pages by interpreting content streams with
//! font encoding information. Also provides text search across documents.

mod extractor;
mod search;

pub use extractor::TextExtractor;
pub use search::{SearchOptions, SearchResult, TextSearch};
