//! Page — represents a single PDF page.

use folio_core::{Matrix2D, Rect};
use folio_cos::{ObjectId, PdfObject};

/// Represents a single PDF page.
#[derive(Debug, Clone)]
pub struct Page {
    /// The object ID of this page.
    id: ObjectId,
    /// The page dictionary.
    dict: PdfObject,
    /// 1-based page number.
    page_num: u32,
}

/// Page rotation values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None = 0,
    Rotate90 = 90,
    Rotate180 = 180,
    Rotate270 = 270,
}

impl Rotation {
    pub fn from_degrees(degrees: i64) -> Self {
        match ((degrees % 360) + 360) % 360 {
            90 => Rotation::Rotate90,
            180 => Rotation::Rotate180,
            270 => Rotation::Rotate270,
            _ => Rotation::None,
        }
    }

    pub fn degrees(&self) -> i32 {
        *self as i32
    }
}

impl Page {
    pub(crate) fn new(id: ObjectId, dict: PdfObject, page_num: u32) -> Self {
        Self { id, dict, page_num }
    }

    /// Get the object ID of this page.
    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// Get the 1-based page number.
    pub fn page_num(&self) -> u32 {
        self.page_num
    }

    /// Get the raw page dictionary.
    pub fn dict(&self) -> &PdfObject {
        &self.dict
    }

    /// Extract a Rect from an array of 4 numbers.
    fn get_rect(&self, key: &[u8]) -> Option<Rect> {
        let arr = self.dict.dict_get(key)?.as_array()?;
        if arr.len() >= 4 {
            Some(Rect::new(
                arr[0].as_f64()?,
                arr[1].as_f64()?,
                arr[2].as_f64()?,
                arr[3].as_f64()?,
            ))
        } else {
            None
        }
    }

    /// Get the media box (required for all pages).
    pub fn media_box(&self) -> Rect {
        self.get_rect(b"MediaBox")
            .unwrap_or_else(|| Rect::new(0.0, 0.0, 612.0, 792.0)) // Default US Letter
    }

    /// Get the crop box (defaults to media box).
    pub fn crop_box(&self) -> Rect {
        self.get_rect(b"CropBox")
            .unwrap_or_else(|| self.media_box())
    }

    /// Get the bleed box (defaults to crop box).
    pub fn bleed_box(&self) -> Rect {
        self.get_rect(b"BleedBox")
            .unwrap_or_else(|| self.crop_box())
    }

    /// Get the trim box (defaults to crop box).
    pub fn trim_box(&self) -> Rect {
        self.get_rect(b"TrimBox").unwrap_or_else(|| self.crop_box())
    }

    /// Get the art box (defaults to crop box).
    pub fn art_box(&self) -> Rect {
        self.get_rect(b"ArtBox").unwrap_or_else(|| self.crop_box())
    }

    /// Get the page rotation.
    pub fn rotation(&self) -> Rotation {
        let degrees = self.dict.dict_get_i64(b"Rotate").unwrap_or(0);
        Rotation::from_degrees(degrees)
    }

    /// Get the effective page width (accounting for rotation and crop box).
    pub fn width(&self) -> f64 {
        let crop = self.crop_box().normalized();
        match self.rotation() {
            Rotation::Rotate90 | Rotation::Rotate270 => crop.height().abs(),
            _ => crop.width().abs(),
        }
    }

    /// Get the effective page height (accounting for rotation and crop box).
    pub fn height(&self) -> f64 {
        let crop = self.crop_box().normalized();
        match self.rotation() {
            Rotation::Rotate90 | Rotation::Rotate270 => crop.width().abs(),
            _ => crop.height().abs(),
        }
    }

    /// Get the number of annotations on this page.
    pub fn num_annots(&self) -> usize {
        self.dict
            .dict_get(b"Annots")
            .and_then(|o| o.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    }

    /// Get the default transformation matrix for this page.
    ///
    /// Maps from default PDF coordinates (origin at bottom-left) to
    /// the page's crop box, accounting for rotation.
    pub fn default_matrix(&self) -> Matrix2D {
        let crop = self.crop_box().normalized();
        let rot = self.rotation();

        let base = Matrix2D::translation(-crop.x1, -crop.y1);

        match rot {
            Rotation::None => base,
            Rotation::Rotate90 => {
                let rotate = Matrix2D::new(0.0, -1.0, 1.0, 0.0, 0.0, crop.width());
                rotate * base
            }
            Rotation::Rotate180 => {
                let rotate = Matrix2D::new(-1.0, 0.0, 0.0, -1.0, crop.width(), crop.height());
                rotate * base
            }
            Rotation::Rotate270 => {
                let rotate = Matrix2D::new(0.0, 1.0, -1.0, 0.0, crop.height(), 0.0);
                rotate * base
            }
        }
    }

    /// Get the resource dictionary for this page.
    pub fn resources(&self) -> Option<&PdfObject> {
        self.dict.dict_get(b"Resources")
    }

    /// Get the content stream reference(s) for this page.
    pub fn contents(&self) -> Option<&PdfObject> {
        self.dict.dict_get(b"Contents")
    }

    /// Get the UserUnit value (default 1.0 = 1/72 inch).
    pub fn user_unit(&self) -> f64 {
        self.dict.dict_get_f64(b"UserUnit").unwrap_or(1.0)
    }
}
