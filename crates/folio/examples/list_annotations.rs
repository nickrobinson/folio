//! List all annotations in a PDF with their types, positions, and contents.
//!
//! Usage:
//!   cargo run -p folio --example list_annotations -- path/to/file.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: list_annotations <path-to-pdf>");
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
    let mut total = 0;

    for page_num in 1..=page_count {
        let page = match doc.get_page(page_num) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let annots = get_page_annotations(&page, doc.cos_mut());
        if annots.is_empty() {
            continue;
        }

        println!("--- Page {} ({} annotations) ---", page_num, annots.len());
        for annot in &annots {
            let rect = annot.rect();
            let type_name = format!("{:?}", annot.annot_type());

            print!(
                "  {:<15} [{:.0},{:.0},{:.0},{:.0}]",
                type_name, rect.x1, rect.y1, rect.x2, rect.y2
            );

            if let Some(contents) = annot.contents() {
                let preview = if contents.len() > 50 {
                    format!("{}...", &contents[..47])
                } else {
                    contents
                };
                print!("  {:?}", preview);
            }

            if let Some(title) = annot.title() {
                print!("  by {:?}", title);
            }

            println!();
        }
        total += annots.len();
    }

    if total == 0 {
        println!("No annotations found in {}", path);
    } else {
        println!("\n{} annotations total", total);
    }
}

fn get_page_annotations(page: &folio_doc::Page, doc: &mut folio_cos::CosDoc) -> Vec<Annot> {
    let annots_obj = match page.dict().dict_get(b"Annots") {
        Some(obj) => obj.clone(),
        None => return Vec::new(),
    };

    let annot_array = match &annots_obj {
        folio_cos::PdfObject::Array(arr) => arr.clone(),
        folio_cos::PdfObject::Reference(id) => {
            match doc.get_object(id.num).ok().flatten().cloned() {
                Some(folio_cos::PdfObject::Array(arr)) => arr,
                _ => return Vec::new(),
            }
        }
        _ => return Vec::new(),
    };

    annot_array
        .iter()
        .filter_map(|annot_ref| {
            let id = annot_ref.as_reference()?;
            Annot::load(id.num, doc).ok()
        })
        .collect()
}
