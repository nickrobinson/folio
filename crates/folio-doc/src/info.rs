//! Document info dictionary.

use folio_cos::PdfObject;
use indexmap::IndexMap;

/// Document metadata from the Info dictionary.
#[derive(Debug, Clone, Default)]
pub struct DocInfo {
    dict: IndexMap<Vec<u8>, PdfObject>,
}

impl DocInfo {
    pub(crate) fn from_dict(dict: IndexMap<Vec<u8>, PdfObject>) -> Self {
        Self { dict }
    }

    fn get_text(&self, key: &[u8]) -> Option<String> {
        self.dict.get(key).and_then(|obj| match obj {
            PdfObject::Str(s) => Some(decode_pdf_text(s)),
            _ => None,
        })
    }

    /// Get the document title.
    pub fn title(&self) -> Option<String> {
        self.get_text(b"Title")
    }

    /// Get the document author.
    pub fn author(&self) -> Option<String> {
        self.get_text(b"Author")
    }

    /// Get the document subject.
    pub fn subject(&self) -> Option<String> {
        self.get_text(b"Subject")
    }

    /// Get the document keywords.
    pub fn keywords(&self) -> Option<String> {
        self.get_text(b"Keywords")
    }

    /// Get the creator (application that created the original document).
    pub fn creator(&self) -> Option<String> {
        self.get_text(b"Creator")
    }

    /// Get the producer (application that created the PDF).
    pub fn producer(&self) -> Option<String> {
        self.get_text(b"Producer")
    }

    /// Get the creation date as a raw string.
    pub fn creation_date(&self) -> Option<String> {
        self.get_text(b"CreationDate")
    }

    /// Get the modification date as a raw string.
    pub fn mod_date(&self) -> Option<String> {
        self.get_text(b"ModDate")
    }
}

/// Decode a PDF text string to a Rust String.
///
/// PDF text strings can be either:
/// - UTF-16BE (starts with BOM: 0xFE 0xFF)
/// - PDFDocEncoding (a superset of ASCII/Latin-1)
fn decode_pdf_text(data: &[u8]) -> String {
    if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        // UTF-16BE
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
        // PDFDocEncoding — for ASCII range this is identical to ASCII
        // For bytes 128-255, PDFDocEncoding has specific mappings
        // but for now we'll use lossy UTF-8 as a reasonable approximation
        String::from_utf8_lossy(data).into_owned()
    }
}
