//! PDF text search — find text patterns across document pages.
//!
//! Supports literal and regex search with configurable modes.

use super::TextExtractor;
use crate::core::{FolioError, Result};
use crate::cos::CosDoc;
use crate::doc::PdfDoc;
use regex::Regex;

/// Search mode flags.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Use regex pattern matching instead of literal string search.
    pub regex: bool,
    /// Case-sensitive search (default: false = case-insensitive).
    pub case_sensitive: bool,
    /// Match whole words only.
    pub whole_word: bool,
    /// First page to search (1-based, default: 1).
    pub start_page: u32,
    /// Last page to search (inclusive, default: last page). 0 = all pages.
    pub end_page: u32,
    /// Maximum number of results to return (0 = unlimited).
    pub max_results: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            regex: false,
            case_sensitive: false,
            whole_word: false,
            start_page: 1,
            end_page: 0,
            max_results: 0,
        }
    }
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn regex(mut self, enabled: bool) -> Self {
        self.regex = enabled;
        self
    }

    pub fn case_sensitive(mut self, enabled: bool) -> Self {
        self.case_sensitive = enabled;
        self
    }

    pub fn whole_word(mut self, enabled: bool) -> Self {
        self.whole_word = enabled;
        self
    }

    pub fn pages(mut self, start: u32, end: u32) -> Self {
        self.start_page = start;
        self.end_page = end;
        self
    }

    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }
}

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 1-based page number where the match was found.
    pub page_num: u32,
    /// The matched text.
    pub match_text: String,
    /// Byte offset of the match within the page's extracted text.
    pub offset: usize,
    /// Text surrounding the match for context.
    pub context: String,
}

/// Search for text across a PDF document.
pub struct TextSearch;

impl TextSearch {
    /// Search for a pattern in a document.
    ///
    /// Returns all matches across the specified page range.
    pub fn search(
        doc: &mut PdfDoc,
        pattern: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let compiled = compile_pattern(pattern, options)?;
        let page_count = doc.page_count()?;

        let start = options.start_page.max(1);
        let end = if options.end_page == 0 || options.end_page > page_count {
            page_count
        } else {
            options.end_page
        };

        let mut results = Vec::new();

        for page_num in start..=end {
            let page = match doc.get_page(page_num) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let text = match TextExtractor::extract_from_page(&page, doc.cos_mut()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if text.is_empty() {
                continue;
            }

            for mat in compiled.find_iter(&text) {
                let match_text = mat.as_str().to_string();
                let offset = mat.start();

                // Build context: ~40 chars before and after
                let ctx_start = text[..offset]
                    .char_indices()
                    .rev()
                    .nth(40)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let ctx_end = text[mat.end()..]
                    .char_indices()
                    .nth(40)
                    .map(|(i, _)| mat.end() + i)
                    .unwrap_or(text.len());
                let context = text[ctx_start..ctx_end].to_string();

                results.push(SearchResult {
                    page_num,
                    match_text,
                    offset,
                    context,
                });

                if options.max_results > 0 && results.len() >= options.max_results {
                    return Ok(results);
                }
            }
        }

        Ok(results)
    }

    /// Quick check: does the pattern appear anywhere in the document?
    pub fn contains(doc: &mut PdfDoc, pattern: &str) -> Result<bool> {
        let options = SearchOptions::new().max_results(1);
        let results = Self::search(doc, pattern, &options)?;
        Ok(!results.is_empty())
    }

    /// Count total occurrences of pattern across all pages.
    pub fn count(doc: &mut PdfDoc, pattern: &str) -> Result<usize> {
        let results = Self::search(doc, pattern, &SearchOptions::new())?;
        Ok(results.len())
    }

    /// Search with regex pattern.
    pub fn search_regex(
        doc: &mut PdfDoc,
        pattern: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let mut opts = options.clone();
        opts.regex = true;
        Self::search(doc, pattern, &opts)
    }
}

/// Compile the search pattern into a regex.
fn compile_pattern(pattern: &str, options: &SearchOptions) -> Result<Regex> {
    let regex_pattern = if options.regex {
        pattern.to_string()
    } else {
        // Escape regex special characters for literal search
        regex::escape(pattern)
    };

    // Apply whole-word matching
    let regex_pattern = if options.whole_word {
        format!(r"\b{}\b", regex_pattern)
    } else {
        regex_pattern
    };

    // Apply case sensitivity
    let regex_pattern = if options.case_sensitive {
        regex_pattern
    } else {
        format!("(?i){}", regex_pattern)
    };

    Regex::new(&regex_pattern)
        .map_err(|e| FolioError::InvalidArgument(format!("Invalid search pattern: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_literal() {
        let opts = SearchOptions::new();
        let re = compile_pattern("hello world", &opts).unwrap();
        assert!(re.is_match("HELLO WORLD")); // case insensitive by default
        assert!(re.is_match("hello world"));
    }

    #[test]
    fn test_compile_case_sensitive() {
        let opts = SearchOptions::new().case_sensitive(true);
        let re = compile_pattern("Hello", &opts).unwrap();
        assert!(re.is_match("Hello"));
        assert!(!re.is_match("hello"));
    }

    #[test]
    fn test_compile_whole_word() {
        let opts = SearchOptions::new().whole_word(true);
        let re = compile_pattern("the", &opts).unwrap();
        assert!(re.is_match("the cat"));
        assert!(!re.is_match("other"));
    }

    #[test]
    fn test_compile_regex() {
        let opts = SearchOptions::new().regex(true);
        let re = compile_pattern(r"\d{3}-\d{4}", &opts).unwrap();
        assert!(re.is_match("Call 555-1234 now"));
        assert!(!re.is_match("no numbers here"));
    }

    #[test]
    fn test_escape_special_chars() {
        let opts = SearchOptions::new();
        let re = compile_pattern("price: $10.00", &opts).unwrap();
        assert!(re.is_match("The price: $10.00 is final"));
    }
}
