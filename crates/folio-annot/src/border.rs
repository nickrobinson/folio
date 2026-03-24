//! Annotation border styles.

use folio_cos::PdfObject;
use indexmap::IndexMap;

/// Border style types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyleType {
    Solid,
    Dashed,
    Beveled,
    Inset,
    Underline,
}

/// Annotation border style.
#[derive(Debug, Clone)]
pub struct BorderStyle {
    pub style: BorderStyleType,
    pub width: f64,
    pub dash_pattern: Vec<f64>,
}

impl BorderStyle {
    /// Parse from a /BS dictionary.
    pub fn from_dict(dict: &IndexMap<Vec<u8>, PdfObject>) -> Self {
        let style = dict
            .get(b"S".as_slice())
            .and_then(|o| o.as_name())
            .map(|n| match n {
                b"S" => BorderStyleType::Solid,
                b"D" => BorderStyleType::Dashed,
                b"B" => BorderStyleType::Beveled,
                b"I" => BorderStyleType::Inset,
                b"U" => BorderStyleType::Underline,
                _ => BorderStyleType::Solid,
            })
            .unwrap_or(BorderStyleType::Solid);

        let width = dict
            .get(b"W".as_slice())
            .and_then(|o| o.as_f64())
            .unwrap_or(1.0);

        let dash_pattern = dict
            .get(b"D".as_slice())
            .and_then(|o| o.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_else(|| vec![3.0]);

        Self {
            style,
            width,
            dash_pattern,
        }
    }

    /// Get the border style from an annotation dictionary.
    pub fn from_annot_dict(dict: &IndexMap<Vec<u8>, PdfObject>) -> Option<Self> {
        let bs = dict.get(b"BS".as_slice())?.as_dict()?;
        Some(Self::from_dict(bs))
    }
}
