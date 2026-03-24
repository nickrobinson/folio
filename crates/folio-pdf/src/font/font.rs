//! PDF font representation — loads font information from PDF dictionaries.

use super::cmap::ToUnicodeCMap;
use super::encoding::Encoding;
use crate::core::Result;
use crate::cos::{CosDoc, PdfObject};
use std::collections::HashMap;

/// PDF font types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontType {
    Type1,
    TrueType,
    Type0,
    Type3,
    CIDFontType0,
    CIDFontType2,
    MMType1,
    Unknown,
}

/// A loaded PDF font with encoding and metrics.
#[derive(Debug)]
pub struct PdfFont {
    /// Font type.
    pub font_type: FontType,
    /// Base font name (e.g., "Helvetica", "ABCDEF+ArialMT").
    pub base_font: String,
    /// Text encoding for simple fonts.
    pub encoding: Encoding,
    /// ToUnicode CMap (if present).
    pub to_unicode: Option<ToUnicodeCMap>,
    /// Character widths: code -> width in 1/1000 units.
    pub widths: HashMap<u32, f64>,
    /// Default width for missing characters.
    pub default_width: f64,
    /// First character code in the Widths array.
    pub first_char: u32,
    /// Whether this is a CID (multi-byte) font.
    pub is_cid: bool,
}

impl PdfFont {
    /// Load a font from a PDF font dictionary.
    pub fn from_dict(dict: &PdfObject, doc: &mut CosDoc) -> Result<Self> {
        let subtype = dict.dict_get_name_str(b"Subtype").unwrap_or_default();
        let base_font = dict.dict_get_name_str(b"BaseFont").unwrap_or_default();

        let font_type = match subtype.as_str() {
            "Type1" => FontType::Type1,
            "TrueType" => FontType::TrueType,
            "Type0" => FontType::Type0,
            "Type3" => FontType::Type3,
            "CIDFontType0" => FontType::CIDFontType0,
            "CIDFontType2" => FontType::CIDFontType2,
            "MMType1" => FontType::MMType1,
            _ => FontType::Unknown,
        };

        let is_cid = matches!(font_type, FontType::Type0);

        // Load encoding
        let encoding = load_encoding(dict, doc);

        // Load ToUnicode CMap
        let to_unicode = load_tounicode(dict, doc);

        // Load widths
        let (widths, first_char, default_width) = if is_cid {
            load_cid_widths(dict, doc)
        } else {
            load_simple_widths(dict)
        };

        Ok(PdfFont {
            font_type,
            base_font,
            encoding,
            to_unicode,
            widths,
            default_width,
            first_char,
            is_cid,
        })
    }

    /// Get the width of a character code in 1/1000 units.
    pub fn char_width(&self, code: u32) -> f64 {
        self.widths
            .get(&code)
            .copied()
            .unwrap_or(self.default_width)
    }

    /// Decode a byte string to Unicode using this font's encoding.
    pub fn decode_text(&self, data: &[u8]) -> String {
        super::encoding::decode_text(data, &self.encoding, self.to_unicode.as_ref())
    }
}

fn load_encoding(dict: &PdfObject, doc: &mut CosDoc) -> Encoding {
    let base_font = dict.dict_get_name_str(b"BaseFont").unwrap_or_default();
    let subtype = dict.dict_get_name_str(b"Subtype").unwrap_or_default();

    // ZapfDingbats and Symbol have their own built-in encodings
    let is_zapf = base_font == "ZapfDingbats" || base_font.ends_with("+ZapfDingbats");
    let is_symbol = base_font == "Symbol" || base_font.ends_with("+Symbol");

    match dict.dict_get(b"Encoding") {
        Some(PdfObject::Name(name)) => Encoding::from_name(name),
        Some(PdfObject::Dict(d)) => {
            let default_base = if is_zapf {
                b"ZapfDingbatsEncoding".as_slice()
            } else if is_symbol {
                b"SymbolEncoding".as_slice()
            } else {
                b"WinAnsiEncoding".as_slice()
            };
            let base_name = d
                .get(b"BaseEncoding".as_slice())
                .and_then(|o| o.as_name())
                .unwrap_or(default_base);
            let mut enc = Encoding::from_name(base_name);

            if let Some(PdfObject::Array(diffs)) = d.get(b"Differences".as_slice()) {
                enc.apply_differences(diffs);
            }

            enc
        }
        Some(PdfObject::Reference(id)) => {
            if let Ok(Some(obj)) = doc.get_object(id.num) {
                let obj = obj.clone();
                return load_encoding_from_obj(&obj, doc, is_zapf, is_symbol);
            }
            Encoding::win_ansi()
        }
        None => {
            // No explicit encoding — choose default based on font type
            if is_zapf {
                Encoding::zapf_dingbats()
            } else if is_symbol {
                Encoding::symbol()
            } else if subtype == "TrueType" && is_subset_font(&base_font) {
                // Subsetted TrueType fonts without encoding often use MacRoman
                // (common in PDFs from Mac-based tools)
                Encoding::mac_roman()
            } else {
                Encoding::win_ansi()
            }
        }
        _ => Encoding::win_ansi(),
    }
}

fn load_encoding_from_obj(
    obj: &PdfObject,
    _doc: &mut CosDoc,
    is_zapf: bool,
    is_symbol: bool,
) -> Encoding {
    match obj {
        PdfObject::Name(name) => Encoding::from_name(name),
        PdfObject::Dict(d) => {
            let default_base = if is_zapf {
                b"ZapfDingbatsEncoding".as_slice()
            } else if is_symbol {
                b"SymbolEncoding".as_slice()
            } else {
                b"WinAnsiEncoding".as_slice()
            };
            let base_name = d
                .get(b"BaseEncoding".as_slice())
                .and_then(|o| o.as_name())
                .unwrap_or(default_base);
            let mut enc = Encoding::from_name(base_name);
            if let Some(PdfObject::Array(diffs)) = d.get(b"Differences".as_slice()) {
                enc.apply_differences(diffs);
            }
            enc
        }
        _ => Encoding::win_ansi(),
    }
}

fn load_tounicode(dict: &PdfObject, doc: &mut CosDoc) -> Option<ToUnicodeCMap> {
    let tu_ref = dict.dict_get(b"ToUnicode")?;

    let stream = match tu_ref {
        PdfObject::Reference(id) => {
            let obj = doc.get_object(id.num).ok()??;
            obj.clone()
        }
        other => other.clone(),
    };

    let stream_data = match &stream {
        PdfObject::Stream(s) => doc.decode_stream(s).ok()?,
        _ => return None,
    };

    ToUnicodeCMap::parse(&stream_data).ok()
}

fn load_simple_widths(dict: &PdfObject) -> (HashMap<u32, f64>, u32, f64) {
    let first_char = dict.dict_get_i64(b"FirstChar").unwrap_or(0) as u32;
    let default_width = dict.dict_get_f64(b"MissingWidth").unwrap_or(1000.0);

    let mut widths = HashMap::new();

    if let Some(PdfObject::Array(w_arr)) = dict.dict_get(b"Widths") {
        for (i, w) in w_arr.iter().enumerate() {
            if let Some(width) = w.as_f64() {
                widths.insert(first_char + i as u32, width);
            }
        }
    }

    (widths, first_char, default_width)
}

fn load_cid_widths(dict: &PdfObject, doc: &mut CosDoc) -> (HashMap<u32, f64>, u32, f64) {
    // For Type0 fonts, widths are in the descendant CIDFont
    let descendant = dict
        .dict_get(b"DescendantFonts")
        .and_then(|o| o.as_array())
        .and_then(|a| a.first())
        .cloned();

    let cid_dict = match descendant {
        Some(PdfObject::Reference(id)) => doc.get_object(id.num).ok().flatten().cloned(),
        Some(obj) => Some(obj),
        None => None,
    };

    let cid_dict = match cid_dict {
        Some(d) => d,
        None => return (HashMap::new(), 0, 1000.0),
    };

    let default_width = cid_dict.dict_get_f64(b"DW").unwrap_or(1000.0);
    let mut widths = HashMap::new();

    // Parse /W array: [cid [w1 w2 ...]] or [cid_start cid_end w]
    if let Some(PdfObject::Array(w_arr)) = cid_dict.dict_get(b"W") {
        let mut i = 0;
        while i < w_arr.len() {
            let cid_start = match w_arr[i].as_i64() {
                Some(n) => n as u32,
                None => {
                    i += 1;
                    continue;
                }
            };

            if i + 1 < w_arr.len() {
                match &w_arr[i + 1] {
                    PdfObject::Array(widths_arr) => {
                        // [cid [w1 w2 w3 ...]]
                        for (j, w) in widths_arr.iter().enumerate() {
                            if let Some(width) = w.as_f64() {
                                widths.insert(cid_start + j as u32, width);
                            }
                        }
                        i += 2;
                    }
                    PdfObject::Integer(_) | PdfObject::Real(_) if i + 2 < w_arr.len() => {
                        // [cid_start cid_end w]
                        let cid_end = w_arr[i + 1].as_i64().unwrap_or(0) as u32;
                        let width = w_arr[i + 2].as_f64().unwrap_or(default_width);
                        for cid in cid_start..=cid_end {
                            widths.insert(cid, width);
                        }
                        i += 3;
                    }
                    _ => {
                        i += 1;
                    }
                }
            } else {
                i += 1;
            }
        }
    }

    (widths, 0, default_width)
}

/// Check if a font name has a subset prefix (e.g., "ABCDEF+FontName").
fn is_subset_font(name: &str) -> bool {
    if name.len() < 8 {
        return false;
    }
    let prefix = &name[..6];
    let has_plus = name.as_bytes().get(6) == Some(&b'+');
    has_plus && prefix.chars().all(|c| c.is_ascii_uppercase())
}
