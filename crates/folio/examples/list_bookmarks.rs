//! List the bookmark (outline) tree of a PDF.
//!
//! Usage:
//!   cargo run -p folio --example list_bookmarks -- path/to/file.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: list_bookmarks <path-to-pdf>");
            std::process::exit(1);
        }
    };

    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut doc = CosDoc::open(data).unwrap_or_else(|e| {
        eprintln!("Cannot open PDF: {}", e);
        std::process::exit(1);
    });

    let bookmarks = Bookmark::get_all(&mut doc).unwrap_or_else(|e| {
        eprintln!("Cannot read bookmarks: {}", e);
        std::process::exit(1);
    });

    if bookmarks.is_empty() {
        println!("No bookmarks found in {}", path);
        return;
    }

    println!("{} bookmarks in {}:\n", bookmarks.len(), path);

    for (bm, depth) in &bookmarks {
        let indent = "  ".repeat(*depth as usize);
        let mut style = Vec::new();
        if bm.is_bold() {
            style.push("bold");
        }
        if bm.is_italic() {
            style.push("italic");
        }
        let style_str = if style.is_empty() {
            String::new()
        } else {
            format!(" [{}]", style.join(", "))
        };

        println!("{}{}{}", indent, bm.title(), style_str);
    }
}
