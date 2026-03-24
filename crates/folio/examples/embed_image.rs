//! Embed an image into a new PDF.
//!
//! Usage:
//!   cargo run -p folio --example embed_image -- image.jpg output.pdf
//!   cargo run -p folio --example embed_image -- photo.png output.pdf

use folio::prelude::*;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: embed_image <image-file> <output.pdf>");
        eprintln!("  Supported formats: JPEG, PNG, GIF, BMP, TIFF");
        std::process::exit(1);
    }

    let image_path = &args[1];
    let output_path = &args[2];

    // Create a new PDF document
    let mut doc = CosDoc::new();

    // Embed the image
    let image = PdfImage::from_file(image_path, &mut doc).unwrap_or_else(|e| {
        eprintln!("Cannot embed image: {}", e);
        std::process::exit(1);
    });

    println!("Image: {}x{} pixels", image.width, image.height);

    // Create a page sized to fit the image (1 pixel = 1 point at 72 DPI)
    // For a more reasonable size, scale to fit within US Letter
    let max_w = 540.0_f64; // 7.5 inches at 72 DPI (with 0.5" margins)
    let max_h = 720.0_f64; // 10 inches at 72 DPI (with 0.5" margins)
    let margin = 36.0; // 0.5 inch

    let scale = (max_w / image.width as f64)
        .min(max_h / image.height as f64)
        .min(1.0);
    let img_w = image.width as f64 * scale;
    let img_h = image.height as f64 * scale;

    let page_w = img_w + 2.0 * margin;
    let page_h = img_h + 2.0 * margin;

    // Create page
    let mut page_dict = indexmap::IndexMap::new();
    page_dict.insert(
        b"Type".to_vec(),
        folio_cos::PdfObject::Name(b"Page".to_vec()),
    );
    page_dict.insert(
        b"MediaBox".to_vec(),
        folio_cos::PdfObject::Array(vec![
            folio_cos::PdfObject::Real(0.0),
            folio_cos::PdfObject::Real(0.0),
            folio_cos::PdfObject::Real(page_w),
            folio_cos::PdfObject::Real(page_h),
        ]),
    );
    let page_id = doc.create_indirect(folio_cos::PdfObject::Dict(page_dict));

    // Create Pages dict
    let mut pages_dict = indexmap::IndexMap::new();
    pages_dict.insert(
        b"Type".to_vec(),
        folio_cos::PdfObject::Name(b"Pages".to_vec()),
    );
    pages_dict.insert(
        b"Kids".to_vec(),
        folio_cos::PdfObject::Array(vec![folio_cos::PdfObject::Reference(page_id)]),
    );
    pages_dict.insert(b"Count".to_vec(), folio_cos::PdfObject::Integer(1));
    let pages_id = doc.create_indirect(folio_cos::PdfObject::Dict(pages_dict));

    // Update page parent
    let page_obj = doc.get_object(page_id.num).unwrap().unwrap().clone();
    let mut pd = page_obj.as_dict().unwrap().clone();
    pd.insert(
        b"Parent".to_vec(),
        folio_cos::PdfObject::Reference(pages_id),
    );
    doc.update_object(page_id.num, folio_cos::PdfObject::Dict(pd));

    // Create Catalog
    let mut catalog_dict = indexmap::IndexMap::new();
    catalog_dict.insert(
        b"Type".to_vec(),
        folio_cos::PdfObject::Name(b"Catalog".to_vec()),
    );
    catalog_dict.insert(b"Pages".to_vec(), folio_cos::PdfObject::Reference(pages_id));
    let catalog_id = doc.create_indirect(folio_cos::PdfObject::Dict(catalog_dict));

    doc.trailer_mut().insert(
        b"Root".to_vec(),
        folio_cos::PdfObject::Reference(catalog_id),
    );

    // Add the image to the page
    let image_rect = Rect::new(margin, margin, margin + img_w, margin + img_h);
    add_image_to_page(page_id.num, &image, image_rect, "Img0", &mut doc).unwrap();

    // Save
    let bytes = doc.save_to_bytes().unwrap();
    std::fs::write(output_path, &bytes).unwrap();

    println!(
        "Created {} ({} bytes) — {:.0}x{:.0} pt page with image",
        output_path,
        bytes.len(),
        page_w,
        page_h
    );
}
