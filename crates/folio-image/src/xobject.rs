//! Read properties from an existing Image XObject in a PDF.

use folio_core::{FolioError, Result};
use folio_cos::{PdfObject, PdfStream};

/// Properties of a PDF Image XObject.
#[derive(Debug, Clone)]
pub struct ImageXObject {
    pub width: u32,
    pub height: u32,
    pub bits_per_component: u32,
    pub color_space: String,
    pub is_image_mask: bool,
    pub is_interpolate: bool,
    pub filters: Vec<String>,
}

impl ImageXObject {
    /// Read image properties from a stream dictionary.
    pub fn from_stream(stream: &PdfStream) -> Result<Self> {
        let dict = &stream.dict;

        let width = dict
            .get(b"Width".as_slice())
            .and_then(|o| o.as_i64())
            .ok_or_else(|| FolioError::InvalidObject("Image missing /Width".into()))?
            as u32;

        let height = dict
            .get(b"Height".as_slice())
            .and_then(|o| o.as_i64())
            .ok_or_else(|| FolioError::InvalidObject("Image missing /Height".into()))?
            as u32;

        let bpc = dict
            .get(b"BitsPerComponent".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(8) as u32;

        let color_space = dict
            .get(b"ColorSpace".as_slice())
            .and_then(|o| o.as_name())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_else(|| "DeviceRGB".into());

        let is_image_mask = dict
            .get(b"ImageMask".as_slice())
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        let is_interpolate = dict
            .get(b"Interpolate".as_slice())
            .and_then(|o| o.as_bool())
            .unwrap_or(false);

        let filters = match dict.get(b"Filter".as_slice()) {
            Some(PdfObject::Name(n)) => vec![String::from_utf8_lossy(n).into_owned()],
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|o| o.as_name().map(|n| String::from_utf8_lossy(n).into_owned()))
                .collect(),
            _ => Vec::new(),
        };

        Ok(Self {
            width,
            height,
            bits_per_component: bpc,
            color_space,
            is_image_mask,
            is_interpolate,
            filters,
        })
    }

    /// Number of color components based on color space name.
    pub fn num_components(&self) -> u32 {
        match self.color_space.as_str() {
            "DeviceGray" => 1,
            "DeviceRGB" => 3,
            "DeviceCMYK" => 4,
            _ => 3, // default assumption
        }
    }
}
