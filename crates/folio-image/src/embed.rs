//! Embed images into a PDF as Image XObjects.

use folio_core::{FolioError, Result};
use folio_cos::{CosDoc, ObjectId, PdfObject, PdfStream};
use indexmap::IndexMap;
use std::path::Path;

/// An image embedded in a PDF document.
#[derive(Debug)]
pub struct PdfImage {
    /// The indirect object ID of the image XObject.
    obj_id: ObjectId,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

impl PdfImage {
    /// Embed an image from a file (JPEG, PNG, GIF, BMP, TIFF).
    ///
    /// JPEG files are embedded directly without re-encoding (passthrough).
    /// Other formats are decoded and re-encoded as Flate-compressed raw RGB.
    pub fn from_file(path: impl AsRef<Path>, doc: &mut CosDoc) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;

        // Detect format from magic bytes
        if is_jpeg(&data) {
            Self::from_jpeg_bytes(&data, doc)
        } else {
            Self::from_image_bytes(&data, doc)
        }
    }

    /// Embed a JPEG directly (passthrough — no re-encoding).
    pub fn from_jpeg_bytes(data: &[u8], doc: &mut CosDoc) -> Result<Self> {
        // Parse JPEG header to get dimensions
        let (width, height) = jpeg_dimensions(data)?;

        let mut dict = IndexMap::new();
        dict.insert(b"Type".to_vec(), PdfObject::Name(b"XObject".to_vec()));
        dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Image".to_vec()));
        dict.insert(b"Width".to_vec(), PdfObject::Integer(width as i64));
        dict.insert(b"Height".to_vec(), PdfObject::Integer(height as i64));
        dict.insert(
            b"ColorSpace".to_vec(),
            PdfObject::Name(b"DeviceRGB".to_vec()),
        );
        dict.insert(b"BitsPerComponent".to_vec(), PdfObject::Integer(8));
        dict.insert(b"Filter".to_vec(), PdfObject::Name(b"DCTDecode".to_vec()));
        dict.insert(b"Length".to_vec(), PdfObject::Integer(data.len() as i64));

        let stream = PdfStream {
            dict,
            data: data.to_vec(),
            decoded: false,
        };

        let obj_id = doc.create_indirect(PdfObject::Stream(stream));

        Ok(Self {
            obj_id,
            width,
            height,
        })
    }

    /// Embed an image from raw bytes (any format the `image` crate supports).
    /// The image is decoded and stored as Flate-compressed raw RGB data.
    pub fn from_image_bytes(data: &[u8], doc: &mut CosDoc) -> Result<Self> {
        let img = image::load_from_memory(data)
            .map_err(|e| FolioError::InvalidArgument(format!("Cannot decode image: {}", e)))?;

        let rgb = img.to_rgb8();
        let width = rgb.width();
        let height = rgb.height();
        let raw_pixels = rgb.into_raw();

        Self::from_raw_rgb(&raw_pixels, width, height, doc)
    }

    /// Embed from raw RGB pixel data (8 bits per component, row-major).
    pub fn from_raw_rgb(pixels: &[u8], width: u32, height: u32, doc: &mut CosDoc) -> Result<Self> {
        // Compress with Flate
        let compressed = folio_filters::flate_encode(pixels)?;

        let mut dict = IndexMap::new();
        dict.insert(b"Type".to_vec(), PdfObject::Name(b"XObject".to_vec()));
        dict.insert(b"Subtype".to_vec(), PdfObject::Name(b"Image".to_vec()));
        dict.insert(b"Width".to_vec(), PdfObject::Integer(width as i64));
        dict.insert(b"Height".to_vec(), PdfObject::Integer(height as i64));
        dict.insert(
            b"ColorSpace".to_vec(),
            PdfObject::Name(b"DeviceRGB".to_vec()),
        );
        dict.insert(b"BitsPerComponent".to_vec(), PdfObject::Integer(8));
        dict.insert(b"Filter".to_vec(), PdfObject::Name(b"FlateDecode".to_vec()));
        dict.insert(
            b"Length".to_vec(),
            PdfObject::Integer(compressed.len() as i64),
        );

        let stream = PdfStream {
            dict,
            data: compressed,
            decoded: false,
        };

        let obj_id = doc.create_indirect(PdfObject::Stream(stream));

        Ok(Self {
            obj_id,
            width,
            height,
        })
    }

    /// Get the indirect object ID of the image XObject.
    pub fn obj_id(&self) -> ObjectId {
        self.obj_id
    }
}

/// Check if data starts with JPEG magic bytes.
fn is_jpeg(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8
}

/// Parse JPEG dimensions from the SOF marker.
fn jpeg_dimensions(data: &[u8]) -> Result<(u32, u32)> {
    let mut i = 2; // Skip FFD8
    while i + 4 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);

        // SOF markers (Start of Frame)
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            if i + 9 < data.len() {
                let height = ((data[i + 5] as u32) << 8) | (data[i + 6] as u32);
                let width = ((data[i + 7] as u32) << 8) | (data[i + 8] as u32);
                return Ok((width, height));
            }
        }

        i += 2 + length;
    }
    Err(FolioError::Parse {
        offset: 0,
        message: "Cannot find JPEG dimensions (no SOF marker)".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_raw_rgb() {
        let mut doc = CosDoc::new();
        // 2x2 red image
        let pixels = vec![
            255, 0, 0, 255, 0, 0, // row 1
            255, 0, 0, 255, 0, 0, // row 2
        ];
        let img = PdfImage::from_raw_rgb(&pixels, 2, 2, &mut doc).unwrap();
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 2);

        // Verify the object was created
        let obj = doc.get_object(img.obj_id().num).unwrap().unwrap();
        assert!(obj.is_stream());
    }

    #[test]
    fn test_is_jpeg() {
        assert!(is_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(!is_jpeg(&[0x89, 0x50, 0x4E, 0x47])); // PNG
        assert!(!is_jpeg(&[0x00, 0x00]));
    }
}
