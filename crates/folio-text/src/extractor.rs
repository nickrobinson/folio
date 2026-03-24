//! Text extraction from PDF pages.
//!
//! Walks the content stream, tracks the text state (font, position),
//! and decodes text strings using the current font's encoding/ToUnicode CMap.

use folio_content::{ContentOp, TextOp, parse_content_stream};
use folio_core::{FolioError, Result};
use folio_cos::{CosDoc, PdfObject};
use folio_font::PdfFont;
use std::collections::HashMap;

/// Extracts text from a PDF page.
pub struct TextExtractor;

impl TextExtractor {
    /// Extract all text from a page as a single string.
    ///
    /// Takes the page's content stream data (already decoded) and its
    /// resource dictionary for font lookups.
    pub fn extract_text(
        content_data: &[u8],
        resources: &PdfObject,
        doc: &mut CosDoc,
    ) -> Result<String> {
        let ops = parse_content_stream(content_data)?;
        let fonts = load_page_fonts(resources, doc)?;

        let mut result = String::new();
        let mut current_font_name: Vec<u8> = Vec::new();
        let mut in_text = false;
        let mut last_was_newline = false;

        for op in &ops {
            match op {
                ContentOp::BeginText => {
                    in_text = true;
                }
                ContentOp::EndText => {
                    in_text = false;
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                        last_was_newline = true;
                    }
                }
                ContentOp::SetFont(name, _size) => {
                    current_font_name = name.clone();
                }
                ContentOp::MoveTextPos(tx, ty) if in_text => {
                    // Significant vertical movement suggests a new line
                    if ty.abs() > 1.0 && !last_was_newline {
                        if !result.is_empty() && !result.ends_with('\n') && !result.ends_with(' ') {
                            result.push('\n');
                            last_was_newline = true;
                        }
                    } else if *tx > 1.0 && !last_was_newline {
                        // Horizontal gap suggests a space
                        if !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\n') {
                            result.push(' ');
                        }
                    }
                }
                ContentOp::MoveTextPosSetLeading(_tx, ty) if in_text => {
                    if ty.abs() > 1.0 && !last_was_newline {
                        if !result.is_empty() && !result.ends_with('\n') && !result.ends_with(' ') {
                            result.push('\n');
                            last_was_newline = true;
                        }
                    }
                }
                ContentOp::NextLine if in_text => {
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                        last_was_newline = true;
                    }
                }
                ContentOp::SetTextMatrix(..) if in_text => {
                    // New text positioning — may need spacing
                }
                ContentOp::ShowText(data) if in_text => {
                    let text = decode_with_font(data, &current_font_name, &fonts);
                    result.push_str(&text);
                    last_was_newline = false;
                }
                ContentOp::ShowTextAdjusted(items) if in_text => {
                    for item in items {
                        match item {
                            TextOp::Text(data) => {
                                let text = decode_with_font(data, &current_font_name, &fonts);
                                result.push_str(&text);
                                last_was_newline = false;
                            }
                            TextOp::Adjustment(adj) => {
                                // Large negative adjustment = space between words
                                // (typical threshold is around -100 to -200)
                                if *adj < -200.0 {
                                    if !result.is_empty()
                                        && !result.ends_with(' ')
                                        && !result.ends_with('\n')
                                    {
                                        result.push(' ');
                                    }
                                }
                            }
                        }
                    }
                }
                ContentOp::NextLineShowText(data) if in_text => {
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    let text = decode_with_font(data, &current_font_name, &fonts);
                    result.push_str(&text);
                    last_was_newline = false;
                }
                ContentOp::SetSpacingNextLineShowText(_, _, data) if in_text => {
                    if !result.is_empty() && !result.ends_with('\n') {
                        result.push('\n');
                    }
                    let text = decode_with_font(data, &current_font_name, &fonts);
                    result.push_str(&text);
                    last_was_newline = false;
                }
                _ => {}
            }
        }

        // Trim trailing whitespace
        let result = result.trim_end().to_string();
        Ok(result)
    }

    /// Extract text from a page using the folio-doc Page object.
    ///
    /// This is the high-level convenience method. It extracts text from the
    /// content stream and also includes text from form field values.
    pub fn extract_from_page(page: &folio_doc::Page, doc: &mut CosDoc) -> Result<String> {
        let mut result = String::new();

        // Extract from content stream
        if let Some(contents) = page.contents() {
            let content_data = resolve_content_stream(contents, doc)?;
            let resources = resolve_resources(page, doc)?;
            let content_text = Self::extract_text(&content_data, &resources, doc)?;
            result.push_str(&content_text);
        }

        // Extract from annotations on this page (form field values + appearance streams)
        let form_text = Self::extract_annotation_text(page, doc);
        if !form_text.is_empty() {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(&form_text);
        }

        Ok(result.trim_end().to_string())
    }

    /// Extract text from annotations on a page.
    ///
    /// This extracts text from:
    /// 1. Widget annotation field values (/V)
    /// 2. Widget appearance streams (/AP/N) for form labels/templates
    fn extract_annotation_text(page: &folio_doc::Page, doc: &mut CosDoc) -> String {
        let annots = match page.dict().dict_get(b"Annots") {
            Some(obj) => obj.clone(),
            None => return String::new(),
        };

        let annot_array = match &annots {
            PdfObject::Array(arr) => arr.clone(),
            PdfObject::Reference(id) => match doc.get_object(id.num).ok().flatten().cloned() {
                Some(PdfObject::Array(arr)) => arr,
                _ => return String::new(),
            },
            _ => return String::new(),
        };

        let mut texts = Vec::new();

        for annot_ref in &annot_array {
            let annot = match annot_ref {
                PdfObject::Reference(id) => match doc.get_object(id.num).ok().flatten().cloned() {
                    Some(obj) => obj,
                    None => continue,
                },
                obj => obj.clone(),
            };

            let subtype = annot.dict_get_name(b"Subtype");
            if subtype != Some(b"Widget") {
                continue;
            }

            // Try field value /V first
            if let Some(value) = annot.dict_get(b"V") {
                let text = match value {
                    PdfObject::Str(s) => decode_pdf_text_string(s),
                    PdfObject::Name(n) => {
                        let name = String::from_utf8_lossy(n).to_string();
                        if name == "Off" { String::new() } else { name }
                    }
                    _ => String::new(),
                };
                let text = text.trim().to_string();
                if !text.is_empty() {
                    texts.push(text);
                    continue; // Field has a value, skip appearance stream
                }
            }

            // Try extracting text from the appearance stream (/AP/N)
            if let Some(text) = extract_appearance_text(&annot, doc) {
                let text = text.trim().to_string();
                if !text.is_empty() {
                    texts.push(text);
                }
            }
        }

        texts.join("\n")
    }

    /// Extract all form field text from an entire document.
    pub fn extract_form_fields(doc: &mut CosDoc) -> Result<Vec<(String, String)>> {
        let mut fields = Vec::new();

        // Get the AcroForm from the catalog
        let catalog_ref = doc
            .trailer()
            .get(b"Root".as_slice())
            .and_then(|o| o.as_reference())
            .ok_or_else(|| FolioError::InvalidObject("No /Root in trailer".into()))?;

        let catalog = doc
            .get_object(catalog_ref.num)?
            .ok_or_else(|| FolioError::InvalidObject("Catalog not found".into()))?
            .clone();

        let acroform_ref = match catalog.dict_get(b"AcroForm") {
            Some(PdfObject::Reference(id)) => doc.get_object(id.num)?.cloned().unwrap_or_default(),
            Some(obj) => obj.clone(),
            None => return Ok(fields),
        };

        let form_fields = match acroform_ref.dict_get(b"Fields") {
            Some(PdfObject::Array(arr)) => arr.clone(),
            _ => return Ok(fields),
        };

        for field_ref in &form_fields {
            Self::collect_field_values(field_ref, doc, &mut fields);
        }

        Ok(fields)
    }

    fn collect_field_values(
        field_ref: &PdfObject,
        doc: &mut CosDoc,
        results: &mut Vec<(String, String)>,
    ) {
        let field = match field_ref {
            PdfObject::Reference(id) => match doc.get_object(id.num).ok().flatten().cloned() {
                Some(obj) => obj,
                None => return,
            },
            obj => obj.clone(),
        };

        // Get field name
        let name = field
            .dict_get(b"T")
            .and_then(|o| o.as_str())
            .map(|s| decode_pdf_text_string(s))
            .unwrap_or_default();

        // Get field value
        if let Some(value) = field.dict_get(b"V") {
            let text = match value {
                PdfObject::Str(s) => decode_pdf_text_string(s),
                PdfObject::Name(n) => String::from_utf8_lossy(n).to_string(),
                _ => String::new(),
            };
            if !text.is_empty() && text != "Off" {
                results.push((name.clone(), text));
            }
        }

        // Recurse into /Kids
        if let Some(PdfObject::Array(kids)) = field.dict_get(b"Kids") {
            for kid in kids {
                Self::collect_field_values(kid, doc, results);
            }
        }
    }
}

/// Decode text bytes using the named font.
fn decode_with_font(data: &[u8], font_name: &[u8], fonts: &HashMap<Vec<u8>, PdfFont>) -> String {
    match fonts.get(font_name) {
        Some(font) => font.decode_text(data),
        None => {
            // No font found — try raw UTF-8, fall back to Latin-1
            String::from_utf8(data.to_vec())
                .unwrap_or_else(|_| data.iter().map(|&b| b as char).collect())
        }
    }
}

/// Load all fonts from a page's Resources dictionary.
fn load_page_fonts(resources: &PdfObject, doc: &mut CosDoc) -> Result<HashMap<Vec<u8>, PdfFont>> {
    let mut fonts = HashMap::new();

    let font_dict = match resources.dict_get(b"Font") {
        Some(PdfObject::Reference(id)) => {
            doc.get_object(id.num)?.cloned().unwrap_or(PdfObject::Null)
        }
        Some(obj) => obj.clone(),
        None => return Ok(fonts),
    };

    let font_entries = match font_dict.as_dict() {
        Some(d) => d.clone(),
        None => return Ok(fonts),
    };

    for (name, value) in &font_entries {
        let font_obj = match value {
            PdfObject::Reference(id) => match doc.get_object(id.num)? {
                Some(obj) => obj.clone(),
                None => continue,
            },
            obj => obj.clone(),
        };

        match PdfFont::from_dict(&font_obj, doc) {
            Ok(font) => {
                fonts.insert(name.clone(), font);
            }
            Err(e) => {
                log::debug!(
                    "Failed to load font {}: {}",
                    String::from_utf8_lossy(name),
                    e
                );
            }
        }
    }

    Ok(fonts)
}

/// Resolve a page's content stream to decoded bytes.
///
/// Content can be a single stream or an array of streams.
fn resolve_content_stream(contents: &PdfObject, doc: &mut CosDoc) -> Result<Vec<u8>> {
    match contents {
        PdfObject::Reference(id) => {
            let obj = doc
                .get_object(id.num)?
                .ok_or_else(|| FolioError::InvalidObject("Content stream not found".into()))?
                .clone();
            resolve_content_stream(&obj, doc)
        }
        PdfObject::Stream(stream) => doc.decode_stream(stream),
        PdfObject::Array(streams) => {
            // Concatenate multiple content streams
            let mut combined = Vec::new();
            for stream_ref in streams {
                let resolved = match stream_ref {
                    PdfObject::Reference(id) => {
                        doc.get_object(id.num)?.cloned().unwrap_or(PdfObject::Null)
                    }
                    other => other.clone(),
                };
                if let PdfObject::Stream(s) = &resolved {
                    let decoded = doc.decode_stream(s)?;
                    combined.extend_from_slice(&decoded);
                    combined.push(b'\n');
                }
            }
            Ok(combined)
        }
        _ => Err(FolioError::InvalidObject(
            "Invalid content stream type".into(),
        )),
    }
}

/// Resolve the resources dictionary for a page.
fn resolve_resources(page: &folio_doc::Page, doc: &mut CosDoc) -> Result<PdfObject> {
    match page.resources() {
        Some(PdfObject::Reference(id)) => {
            Ok(doc.get_object(id.num)?.cloned().unwrap_or(PdfObject::Null))
        }
        Some(obj) => Ok(obj.clone()),
        None => Ok(PdfObject::Null),
    }
}

/// Decode a PDF text string (handles UTF-16BE BOM and PDFDocEncoding).
fn decode_pdf_text_string(data: &[u8]) -> String {
    if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        // UTF-16BE with BOM
        let mut chars = Vec::new();
        let mut i = 2;
        while i + 1 < data.len() {
            let code = ((data[i] as u16) << 8) | (data[i + 1] as u16);
            chars.push(code);
            i += 2;
        }
        String::from_utf16_lossy(&chars)
    } else if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        // UTF-8 with BOM
        String::from_utf8_lossy(&data[3..]).into_owned()
    } else {
        // PDFDocEncoding (superset of Latin-1 for most practical purposes)
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(data);
        decoded.into_owned()
    }
}

/// Extract text from an annotation's appearance stream (/AP/N).
fn extract_appearance_text(annot: &PdfObject, doc: &mut CosDoc) -> Option<String> {
    // Get /AP dict
    let ap = annot.dict_get(b"AP")?;
    let ap_dict = match ap {
        PdfObject::Reference(id) => doc.get_object(id.num).ok()??.clone(),
        obj => obj.clone(),
    };

    // Get /N (normal appearance) — can be a stream or a dict of streams
    let n_obj = match ap_dict.dict_get(b"N")? {
        PdfObject::Reference(id) => doc.get_object(id.num).ok()??.clone(),
        obj => obj.clone(),
    };

    // If it's a stream, extract text from it
    let stream = match &n_obj {
        PdfObject::Stream(s) => s,
        _ => return None,
    };

    let decoded = doc.decode_stream(stream).ok()?;
    if decoded.is_empty() {
        return None;
    }

    // Get resources from the appearance stream's dict
    let resources = match stream.dict.get(b"Resources".as_slice()) {
        Some(PdfObject::Reference(id)) => doc.get_object(id.num).ok()??.clone(),
        Some(obj) => obj.clone(),
        None => PdfObject::Null,
    };

    TextExtractor::extract_text(&decoded, &resources, doc).ok()
}
