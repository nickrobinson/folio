//! Extract text from every page of a PDF.
//!
//! Usage:
//!   cargo run -p folio --example extract_text -- path/to/file.pdf
//!   cargo run -p folio --example extract_text -- path/to/file.pdf 3    # page 3 only

use folio::prelude::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = match args.get(1) {
        Some(p) => p.clone(),
        None => {
            eprintln!("Usage: extract_text <path-to-pdf> [page-number]");
            std::process::exit(1);
        }
    };
    let specific_page: Option<u32> = args.get(2).and_then(|s| s.parse().ok());

    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut doc = PdfDoc::open_from_bytes(data).unwrap_or_else(|e| {
        eprintln!("Cannot open PDF: {}", e);
        std::process::exit(1);
    });

    let page_count = doc.page_count().unwrap_or(0);
    let (start, end) = match specific_page {
        Some(p) => (p, p),
        None => (1, page_count),
    };

    for page_num in start..=end {
        let page = match doc.get_page(page_num) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Cannot get page {}: {}", page_num, e);
                continue;
            }
        };

        let text = match TextExtractor::extract_from_page(&page, doc.cos_mut()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Cannot extract text from page {}: {}", page_num, e);
                continue;
            }
        };

        if page_count > 1 {
            println!("=== Page {} ===", page_num);
        }
        println!("{}", text);
        if page_count > 1 && page_num < end {
            println!();
        }
    }
}
