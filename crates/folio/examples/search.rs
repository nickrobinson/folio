//! Search for text in a PDF.
//!
//! Usage:
//!   cargo run -p folio --example search -- file.pdf "search term"
//!   cargo run -p folio --example search -- file.pdf "pattern" --regex
//!   cargo run -p folio --example search -- file.pdf "word" --whole-word
//!   cargo run -p folio --example search -- file.pdf "Term" --case-sensitive

use folio::prelude::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: search <pdf-file> <pattern> [--regex] [--whole-word] [--case-sensitive]");
        std::process::exit(1);
    }

    let path = &args[1];
    let pattern = &args[2];

    let mut options = SearchOptions::new();
    for arg in &args[3..] {
        match arg.as_str() {
            "--regex" => options = options.regex(true),
            "--whole-word" => options = options.whole_word(true),
            "--case-sensitive" => options = options.case_sensitive(true),
            _ => {
                eprintln!("Unknown option: {}", arg);
                std::process::exit(1);
            }
        }
    }

    let data = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut doc = PdfDoc::open_from_bytes(data).unwrap_or_else(|e| {
        eprintln!("Cannot open PDF: {}", e);
        std::process::exit(1);
    });

    let results = TextSearch::search(&mut doc, pattern, &options).unwrap_or_else(|e| {
        eprintln!("Search failed: {}", e);
        std::process::exit(1);
    });

    if results.is_empty() {
        println!("No matches found for {:?}", pattern);
        return;
    }

    println!("{} matches for {:?}:\n", results.len(), pattern);
    for result in &results {
        let ctx = result.context.replace('\n', " ").replace('\r', "");
        let ctx = ctx.trim();
        println!(
            "  Page {}, offset {}: ...{}...",
            result.page_num, result.offset, ctx
        );
    }
}
