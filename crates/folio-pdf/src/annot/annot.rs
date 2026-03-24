//! Base annotation type wrapping a PDF annotation dictionary.

use crate::core::{ColorPt, FolioError, PdfDate, Rect, Result};
use crate::cos::{CosDoc, ObjectId, PdfObject};
use indexmap::IndexMap;

/// PDF annotation types (ISO 32000-2 Table 169).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotType {
    Text,
    Link,
    FreeText,
    Line,
    Square,
    Circle,
    Polygon,
    PolyLine,
    Highlight,
    Underline,
    Squiggly,
    StrikeOut,
    Stamp,
    Caret,
    Ink,
    Popup,
    FileAttachment,
    Sound,
    Movie,
    Widget,
    Screen,
    PrinterMark,
    TrapNet,
    Watermark,
    ThreeD,
    Redact,
    Projection,
    RichMedia,
    Unknown,
}

impl AnnotType {
    /// Parse from a PDF /Subtype name.
    pub fn from_name(name: &[u8]) -> Self {
        match name {
            b"Text" => Self::Text,
            b"Link" => Self::Link,
            b"FreeText" => Self::FreeText,
            b"Line" => Self::Line,
            b"Square" => Self::Square,
            b"Circle" => Self::Circle,
            b"Polygon" => Self::Polygon,
            b"PolyLine" => Self::PolyLine,
            b"Highlight" => Self::Highlight,
            b"Underline" => Self::Underline,
            b"Squiggly" => Self::Squiggly,
            b"StrikeOut" => Self::StrikeOut,
            b"Stamp" => Self::Stamp,
            b"Caret" => Self::Caret,
            b"Ink" => Self::Ink,
            b"Popup" => Self::Popup,
            b"FileAttachment" => Self::FileAttachment,
            b"Sound" => Self::Sound,
            b"Movie" => Self::Movie,
            b"Widget" => Self::Widget,
            b"Screen" => Self::Screen,
            b"PrinterMark" => Self::PrinterMark,
            b"TrapNet" => Self::TrapNet,
            b"Watermark" => Self::Watermark,
            b"3D" => Self::ThreeD,
            b"Redact" => Self::Redact,
            b"Projection" => Self::Projection,
            b"RichMedia" => Self::RichMedia,
            _ => Self::Unknown,
        }
    }

    /// Get the PDF /Subtype name for this annotation type.
    pub fn to_name(&self) -> &'static [u8] {
        match self {
            Self::Text => b"Text",
            Self::Link => b"Link",
            Self::FreeText => b"FreeText",
            Self::Line => b"Line",
            Self::Square => b"Square",
            Self::Circle => b"Circle",
            Self::Polygon => b"Polygon",
            Self::PolyLine => b"PolyLine",
            Self::Highlight => b"Highlight",
            Self::Underline => b"Underline",
            Self::Squiggly => b"Squiggly",
            Self::StrikeOut => b"StrikeOut",
            Self::Stamp => b"Stamp",
            Self::Caret => b"Caret",
            Self::Ink => b"Ink",
            Self::Popup => b"Popup",
            Self::FileAttachment => b"FileAttachment",
            Self::Sound => b"Sound",
            Self::Movie => b"Movie",
            Self::Widget => b"Widget",
            Self::Screen => b"Screen",
            Self::PrinterMark => b"PrinterMark",
            Self::TrapNet => b"TrapNet",
            Self::Watermark => b"Watermark",
            Self::ThreeD => b"3D",
            Self::Redact => b"Redact",
            Self::Projection => b"Projection",
            Self::RichMedia => b"RichMedia",
            Self::Unknown => b"Unknown",
        }
    }

    /// Whether this is a markup annotation type.
    pub fn is_markup(&self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::FreeText
                | Self::Line
                | Self::Square
                | Self::Circle
                | Self::Polygon
                | Self::PolyLine
                | Self::Highlight
                | Self::Underline
                | Self::Squiggly
                | Self::StrikeOut
                | Self::Stamp
                | Self::Caret
                | Self::Ink
                | Self::Sound
                | Self::FileAttachment
                | Self::Redact
        )
    }
}

bitflags::bitflags! {
    /// Annotation flags (ISO 32000-2 Table 170).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AnnotFlags: u32 {
        const INVISIBLE = 1 << 0;
        const HIDDEN = 1 << 1;
        const PRINT = 1 << 2;
        const NO_ZOOM = 1 << 3;
        const NO_ROTATE = 1 << 4;
        const NO_VIEW = 1 << 5;
        const READ_ONLY = 1 << 6;
        const LOCKED = 1 << 7;
        const TOGGLE_NO_VIEW = 1 << 8;
        const LOCKED_CONTENTS = 1 << 9;
    }
}

/// A PDF annotation backed by a dictionary object.
#[derive(Debug, Clone)]
pub struct Annot {
    /// The annotation dictionary.
    dict: IndexMap<Vec<u8>, PdfObject>,
    /// Object ID if this is an indirect object.
    id: Option<ObjectId>,
}

impl Annot {
    /// Create an Annot from a PDF dictionary.
    pub fn from_dict(dict: IndexMap<Vec<u8>, PdfObject>, id: Option<ObjectId>) -> Self {
        Self { dict, id }
    }

    /// Load an annotation from a document by object number.
    pub fn load(obj_num: u32, doc: &mut CosDoc) -> Result<Self> {
        let obj = doc
            .get_object(obj_num)?
            .ok_or_else(|| FolioError::InvalidObject(format!("Annotation {} not found", obj_num)))?
            .clone();
        let dict = obj
            .as_dict()
            .ok_or_else(|| FolioError::InvalidObject("Annotation is not a dict".into()))?
            .clone();
        Ok(Self {
            dict,
            id: Some(ObjectId::new(obj_num, 0)),
        })
    }

    /// Create a new annotation dictionary.
    pub fn create(annot_type: AnnotType, rect: Rect) -> Self {
        let mut dict = IndexMap::new();
        dict.insert(b"Type".to_vec(), PdfObject::Name(b"Annot".to_vec()));
        dict.insert(
            b"Subtype".to_vec(),
            PdfObject::Name(annot_type.to_name().to_vec()),
        );
        dict.insert(
            b"Rect".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(rect.x1),
                PdfObject::Real(rect.y1),
                PdfObject::Real(rect.x2),
                PdfObject::Real(rect.y2),
            ]),
        );
        Self { dict, id: None }
    }

    /// Get the raw dictionary.
    pub fn dict(&self) -> &IndexMap<Vec<u8>, PdfObject> {
        &self.dict
    }

    /// Get a mutable reference to the dictionary.
    pub fn dict_mut(&mut self) -> &mut IndexMap<Vec<u8>, PdfObject> {
        &mut self.dict
    }

    /// Get the object ID (if loaded from a document).
    pub fn id(&self) -> Option<ObjectId> {
        self.id
    }

    /// Get the annotation type.
    pub fn annot_type(&self) -> AnnotType {
        self.dict
            .get(b"Subtype".as_slice())
            .and_then(|o| o.as_name())
            .map(AnnotType::from_name)
            .unwrap_or(AnnotType::Unknown)
    }

    /// Get the annotation rectangle.
    pub fn rect(&self) -> Rect {
        extract_rect(&self.dict, b"Rect").unwrap_or_default()
    }

    /// Set the annotation rectangle.
    pub fn set_rect(&mut self, rect: Rect) {
        self.dict.insert(
            b"Rect".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(rect.x1),
                PdfObject::Real(rect.y1),
                PdfObject::Real(rect.x2),
                PdfObject::Real(rect.y2),
            ]),
        );
    }

    /// Get the annotation flags.
    pub fn flags(&self) -> AnnotFlags {
        let bits = self
            .dict
            .get(b"F".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as u32;
        AnnotFlags::from_bits_truncate(bits)
    }

    /// Set the annotation flags.
    pub fn set_flags(&mut self, flags: AnnotFlags) {
        self.dict
            .insert(b"F".to_vec(), PdfObject::Integer(flags.bits() as i64));
    }

    /// Get the contents (text displayed for the annotation or alternate description).
    pub fn contents(&self) -> Option<String> {
        self.dict
            .get(b"Contents".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
    }

    /// Set the annotation contents.
    pub fn set_contents(&mut self, text: &str) {
        self.dict.insert(
            b"Contents".to_vec(),
            PdfObject::Str(text.as_bytes().to_vec()),
        );
    }

    /// Get the annotation name (unique ID within the page).
    pub fn name(&self) -> Option<String> {
        self.dict
            .get(b"NM".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
    }

    /// Get the modification date.
    pub fn modified_date(&self) -> Option<PdfDate> {
        self.dict
            .get(b"M".as_slice())
            .and_then(|o| o.as_str())
            .and_then(|s| PdfDate::parse(&decode_text(s)))
    }

    /// Get the color (used for border, background, or title bar).
    pub fn color(&self) -> Option<ColorPt> {
        let arr = self.dict.get(b"C".as_slice())?.as_array()?;
        match arr.len() {
            0 => Some(ColorPt::new(0.0, 0.0, 0.0, 0.0)), // transparent
            1 => Some(ColorPt::gray(arr[0].as_f64()?)),
            3 => Some(ColorPt::rgb(
                arr[0].as_f64()?,
                arr[1].as_f64()?,
                arr[2].as_f64()?,
            )),
            4 => Some(ColorPt::cmyk(
                arr[0].as_f64()?,
                arr[1].as_f64()?,
                arr[2].as_f64()?,
                arr[3].as_f64()?,
            )),
            _ => None,
        }
    }

    /// Set the annotation color.
    pub fn set_color(&mut self, color: ColorPt) {
        self.dict.insert(
            b"C".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Real(color.c0),
                PdfObject::Real(color.c1),
                PdfObject::Real(color.c2),
            ]),
        );
    }

    // --- Markup annotation properties ---

    /// Get the title (author) — markup annotations only.
    pub fn title(&self) -> Option<String> {
        self.dict
            .get(b"T".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
    }

    /// Get the subject — markup annotations only.
    pub fn subject(&self) -> Option<String> {
        self.dict
            .get(b"Subj".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
    }

    /// Get the opacity (0.0 = transparent, 1.0 = opaque) — markup annotations only.
    pub fn opacity(&self) -> f64 {
        self.dict
            .get(b"CA".as_slice())
            .and_then(|o| o.as_f64())
            .unwrap_or(1.0)
    }

    /// Get the creation date — markup annotations only.
    pub fn creation_date(&self) -> Option<PdfDate> {
        self.dict
            .get(b"CreationDate".as_slice())
            .and_then(|o| o.as_str())
            .and_then(|s| PdfDate::parse(&decode_text(s)))
    }

    /// Get the popup annotation reference — markup annotations only.
    pub fn popup(&self) -> Option<ObjectId> {
        self.dict
            .get(b"Popup".as_slice())
            .and_then(|o| o.as_reference())
    }

    /// Convert to a PdfObject::Dict for saving.
    pub fn to_pdf_object(&self) -> PdfObject {
        PdfObject::Dict(self.dict.clone())
    }
}

/// Extract a Rect from a dictionary key.
fn extract_rect(dict: &IndexMap<Vec<u8>, PdfObject>, key: &[u8]) -> Option<Rect> {
    let arr = dict.get(key)?.as_array()?;
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

/// Decode a PDF text string.
fn decode_text(data: &[u8]) -> String {
    if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        let mut chars = Vec::new();
        let mut i = 2;
        while i + 1 < data.len() {
            chars.push(((data[i] as u16) << 8) | (data[i + 1] as u16));
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    } else {
        String::from_utf8_lossy(data).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_annotation() {
        let annot = Annot::create(AnnotType::Highlight, Rect::new(100.0, 200.0, 300.0, 220.0));
        assert_eq!(annot.annot_type(), AnnotType::Highlight);
        assert_eq!(annot.rect(), Rect::new(100.0, 200.0, 300.0, 220.0));
        assert!(annot.annot_type().is_markup());
    }

    #[test]
    fn test_annotation_flags() {
        let mut annot = Annot::create(AnnotType::Text, Rect::new(0.0, 0.0, 50.0, 50.0));
        annot.set_flags(AnnotFlags::PRINT | AnnotFlags::LOCKED);
        let flags = annot.flags();
        assert!(flags.contains(AnnotFlags::PRINT));
        assert!(flags.contains(AnnotFlags::LOCKED));
        assert!(!flags.contains(AnnotFlags::HIDDEN));
    }

    #[test]
    fn test_annotation_properties() {
        let mut annot = Annot::create(AnnotType::Text, Rect::new(0.0, 0.0, 50.0, 50.0));
        annot.set_contents("Hello World");
        annot.set_color(ColorPt::rgb(1.0, 0.0, 0.0));
        assert_eq!(annot.contents(), Some("Hello World".into()));
    }

    #[test]
    fn test_annot_type_names() {
        assert_eq!(AnnotType::from_name(b"Highlight"), AnnotType::Highlight);
        assert_eq!(AnnotType::from_name(b"Widget"), AnnotType::Widget);
        assert_eq!(AnnotType::from_name(b"Unknown"), AnnotType::Unknown);
        assert!(AnnotType::Text.is_markup());
        assert!(!AnnotType::Link.is_markup());
        assert!(!AnnotType::Widget.is_markup());
    }
}
