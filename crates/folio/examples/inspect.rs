//! Inspect a PDF file — prints page count, dimensions, metadata, and structure.
//!
//! Usage:
//!   cargo run -p folio --example inspect -- path/to/file.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: inspect <path-to-pdf>");
            std::process::exit(1);
        }
    };

    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {}", path, e);
        std::process::exit(1);
    });

    let mut doc = PdfDoc::open_from_bytes(data).unwrap_or_else(|e| {
        eprintln!("Cannot open PDF: {}", e);
        std::process::exit(1);
    });

    let page_count = doc.page_count().unwrap_or(0);
    println!("File: {}", path);
    println!("Pages: {}", page_count);

    // Metadata
    if let Ok(info) = doc.doc_info() {
        if let Some(title) = info.title() {
            println!("Title: {}", title);
        }
        if let Some(author) = info.author() {
            println!("Author: {}", author);
        }
        if let Some(creator) = info.creator() {
            println!("Creator: {}", creator);
        }
        if let Some(producer) = info.producer() {
            println!("Producer: {}", producer);
        }
    }

    // Pages
    println!("\n--- Pages ---");
    for i in 1..=page_count.min(20) {
        if let Ok(page) = doc.get_page(i) {
            let mb = page.media_box();
            let rot = page.rotation().degrees();
            println!(
                "  Page {}: {:.0}x{:.0} pt (MediaBox [{:.0} {:.0} {:.0} {:.0}]) rotation={}°",
                i,
                page.width(),
                page.height(),
                mb.x1,
                mb.y1,
                mb.x2,
                mb.y2,
                rot
            );
        }
    }
    if page_count > 20 {
        println!("  ... ({} more pages)", page_count - 20);
    }
}
