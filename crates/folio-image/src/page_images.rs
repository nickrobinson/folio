//! Add images to PDF pages by writing content stream operators.

use crate::embed::PdfImage;
use folio_core::{Rect, Result};
use folio_cos::{CosDoc, PdfObject, PdfStream};
use indexmap::IndexMap;

/// Add an image to a page at the specified position and size.
///
/// This creates the content stream operators (`q`, `cm`, `Do`, `Q`) to paint
/// the image, registers it in the page's Resources, and appends the content
/// to the page.
///
/// `rect` specifies where to place the image on the page (in PDF points).
pub fn add_image_to_page(
    page_obj_num: u32,
    image: &PdfImage,
    rect: Rect,
    resource_name: &str,
    doc: &mut CosDoc,
) -> Result<()> {
    // Build the content stream: q w 0 0 h x y cm /Name Do Q
    let content = format!(
        "q {} 0 0 {} {} {} cm /{} Do Q\n",
        rect.width(),
        rect.height(),
        rect.x1,
        rect.y1,
        resource_name
    );

    // Create the content stream object
    let content_bytes = content.into_bytes();
    let mut content_dict = IndexMap::new();
    content_dict.insert(
        b"Length".to_vec(),
        PdfObject::Integer(content_bytes.len() as i64),
    );
    let content_stream = PdfStream {
        dict: content_dict,
        data: content_bytes,
        decoded: true,
    };
    let content_id = doc.create_indirect(PdfObject::Stream(content_stream));

    // Get the page dict and update it
    let page = doc
        .get_object(page_obj_num)?
        .ok_or_else(|| folio_core::FolioError::InvalidObject("Page not found".into()))?
        .clone();

    let mut page_dict = page.as_dict().cloned().unwrap_or_default();

    // Add image to Resources/XObject
    let mut resources = page_dict
        .get(b"Resources".as_slice())
        .and_then(|o| o.as_dict())
        .cloned()
        .unwrap_or_default();

    let mut xobjects = resources
        .get(b"XObject".as_slice())
        .and_then(|o| o.as_dict())
        .cloned()
        .unwrap_or_default();

    xobjects.insert(
        resource_name.as_bytes().to_vec(),
        PdfObject::Reference(image.obj_id()),
    );
    resources.insert(b"XObject".to_vec(), PdfObject::Dict(xobjects));
    page_dict.insert(b"Resources".to_vec(), PdfObject::Dict(resources));

    // Append content stream reference to page's /Contents
    let mut contents = match page_dict.get(b"Contents".as_slice()) {
        Some(PdfObject::Array(arr)) => arr.clone(),
        Some(PdfObject::Reference(id)) => vec![PdfObject::Reference(*id)],
        _ => Vec::new(),
    };
    contents.push(PdfObject::Reference(content_id));
    page_dict.insert(b"Contents".to_vec(), PdfObject::Array(contents));

    doc.update_object(page_obj_num, PdfObject::Dict(page_dict));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::PdfImage;

    #[test]
    fn test_add_image_to_page() {
        let mut doc = CosDoc::new();

        // Create a minimal page
        let mut page_dict = IndexMap::new();
        page_dict.insert(b"Type".to_vec(), PdfObject::Name(b"Page".to_vec()));
        page_dict.insert(
            b"MediaBox".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(612.0),
                PdfObject::Real(792.0),
            ]),
        );
        let page_id = doc.create_indirect(PdfObject::Dict(page_dict));

        // Create a small image
        let pixels = vec![255u8; 3 * 10 * 10]; // 10x10 white
        let image = PdfImage::from_raw_rgb(&pixels, 10, 10, &mut doc).unwrap();

        // Add image to page
        add_image_to_page(
            page_id.num,
            &image,
            Rect::new(100.0, 600.0, 200.0, 700.0),
            "Img0",
            &mut doc,
        )
        .unwrap();

        // Verify the page now has Contents and Resources/XObject
        let page = doc.get_object(page_id.num).unwrap().unwrap();
        assert!(page.dict_get(b"Contents").is_some());
        let resources = page.dict_get(b"Resources").unwrap();
        let xobjects = resources.dict_get(b"XObject").unwrap();
        assert!(xobjects.dict_get(b"Img0").is_some());
    }
}
