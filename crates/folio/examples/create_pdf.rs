//! Create a simple PDF from scratch.
//!
//! Usage:
//!   cargo run -p folio --example create_pdf -- output.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let output = env::args().nth(1).unwrap_or_else(|| "output.pdf".into());

    let mut doc = PdfDoc::new().unwrap();

    // US Letter page
    doc.create_page(Rect::new(0.0, 0.0, 612.0, 792.0)).unwrap();

    // A4 page
    doc.create_page(Rect::new(0.0, 0.0, 595.0, 842.0)).unwrap();

    // A4 Landscape
    doc.create_page(Rect::new(0.0, 0.0, 842.0, 595.0)).unwrap();

    let bytes = doc.save_to_bytes().unwrap();
    std::fs::write(&output, &bytes).unwrap();

    println!("Created {} ({} bytes, {} pages)", output, bytes.len(), 3);
}
