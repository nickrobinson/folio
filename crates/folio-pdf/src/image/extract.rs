//! Extract images from PDF pages.

use super::xobject::ImageXObject;
use crate::core::Result;
use crate::cos::{CosDoc, PdfObject};

/// Info about an extracted image.
#[derive(Debug)]
pub struct ExtractedImage {
    /// Image properties.
    pub info: ImageXObject,
    /// Resource name (e.g., "Im0").
    pub name: String,
    /// Raw (possibly compressed) image data.
    pub raw_data: Vec<u8>,
    /// Decoded (uncompressed) pixel data, if available.
    pub decoded_data: Option<Vec<u8>>,
}

/// Extract all images from a page's resources.
pub fn extract_images_from_resources(
    resources: &PdfObject,
    doc: &mut CosDoc,
) -> Result<Vec<ExtractedImage>> {
    let xobject_dict = match resources.dict_get(b"XObject") {
        Some(PdfObject::Reference(id)) => doc.get_object(id.num)?.cloned().unwrap_or_default(),
        Some(obj) => obj.clone(),
        None => return Ok(Vec::new()),
    };

    let entries = match xobject_dict.as_dict() {
        Some(d) => d.clone(),
        None => return Ok(Vec::new()),
    };

    let mut images = Vec::new();

    for (name, value) in &entries {
        let obj = match value {
            PdfObject::Reference(id) => match doc.get_object(id.num)?.cloned() {
                Some(o) => o,
                None => continue,
            },
            obj => obj.clone(),
        };

        let stream = match &obj {
            PdfObject::Stream(s) => s,
            _ => continue,
        };

        // Check if this is an Image XObject
        let subtype = stream
            .dict
            .get(b"Subtype".as_slice())
            .and_then(|o| o.as_name());
        if subtype != Some(b"Image") {
            continue;
        }

        let info = match ImageXObject::from_stream(stream) {
            Ok(i) => i,
            Err(_) => continue,
        };

        let decoded = doc.decode_stream(stream).ok();

        images.push(ExtractedImage {
            info,
            name: String::from_utf8_lossy(name).into_owned(),
            raw_data: stream.data.clone(),
            decoded_data: decoded,
        });
    }

    Ok(images)
}

/// Convenience: extract image by resource name from a page's resources.
pub fn extract_image(
    resources: &PdfObject,
    name: &str,
    doc: &mut CosDoc,
) -> Result<Option<ExtractedImage>> {
    let images = extract_images_from_resources(resources, doc)?;
    Ok(images.into_iter().find(|img| img.name == name))
}
