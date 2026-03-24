//! PDF form field representation.

use folio_cos::{ObjectId, PdfObject};
use indexmap::IndexMap;

/// Form field types (ISO 32000-2 Table 226).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// Push button (no value).
    Button,
    /// Check box.
    CheckBox,
    /// Radio button.
    Radio,
    /// Text input field.
    Text,
    /// Choice field (list box or combo box).
    Choice,
    /// Digital signature field.
    Signature,
    /// Unknown field type.
    Unknown,
}

impl FieldType {
    /// Parse from /FT name and flags.
    pub fn from_ft_and_flags(ft: &[u8], flags: u32) -> Self {
        match ft {
            b"Btn" => {
                if flags & (1 << 16) != 0 {
                    // Pushbutton flag
                    Self::Button
                } else if flags & (1 << 15) != 0 {
                    // Radio flag
                    Self::Radio
                } else {
                    Self::CheckBox
                }
            }
            b"Tx" => Self::Text,
            b"Ch" => Self::Choice,
            b"Sig" => Self::Signature,
            _ => Self::Unknown,
        }
    }
}

bitflags::bitflags! {
    /// Field flags (ISO 32000-2 Tables 227-230).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FieldFlags: u32 {
        const READ_ONLY = 1 << 0;
        const REQUIRED = 1 << 1;
        const NO_EXPORT = 1 << 2;
        // Button-specific
        const NO_TOGGLE_TO_OFF = 1 << 14;
        const RADIO = 1 << 15;
        const PUSHBUTTON = 1 << 16;
        const RADIOS_IN_UNISON = 1 << 25;
        // Text-specific
        const MULTILINE = 1 << 12;
        const PASSWORD = 1 << 13;
        const FILE_SELECT = 1 << 20;
        const DO_NOT_SPELL_CHECK = 1 << 22;
        const DO_NOT_SCROLL = 1 << 23;
        const COMB = 1 << 24;
        const RICH_TEXT = 1 << 25;
        // Choice-specific
        const COMBO = 1 << 17;
        const EDIT = 1 << 18;
        const SORT = 1 << 19;
        const MULTI_SELECT = 1 << 21;
        const COMMIT_ON_SEL_CHANGE = 1 << 26;
    }
}

/// A PDF form field.
#[derive(Debug, Clone)]
pub struct Field {
    /// The field dictionary.
    dict: IndexMap<Vec<u8>, PdfObject>,
    /// Object ID.
    id: Option<ObjectId>,
    /// Fully qualified field name (built by walking parent chain).
    full_name: String,
}

impl Field {
    /// Create a Field from a dictionary and optional parent name.
    pub fn from_dict(
        dict: IndexMap<Vec<u8>, PdfObject>,
        id: Option<ObjectId>,
        parent_name: &str,
    ) -> Self {
        let partial_name = dict
            .get(b"T".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
            .unwrap_or_default();

        let full_name = if parent_name.is_empty() {
            partial_name
        } else if partial_name.is_empty() {
            parent_name.to_string()
        } else {
            format!("{}.{}", parent_name, partial_name)
        };

        Self {
            dict,
            id,
            full_name,
        }
    }

    /// Get the object ID.
    pub fn id(&self) -> Option<ObjectId> {
        self.id
    }

    /// Get the fully qualified field name.
    pub fn name(&self) -> &str {
        &self.full_name
    }

    /// Get the partial field name (/T).
    pub fn partial_name(&self) -> String {
        self.dict
            .get(b"T".as_slice())
            .and_then(|o| o.as_str())
            .map(|s| decode_text(s))
            .unwrap_or_default()
    }

    /// Get the field type.
    pub fn field_type(&self) -> FieldType {
        let ft = self
            .dict
            .get(b"FT".as_slice())
            .and_then(|o| o.as_name())
            .unwrap_or(b"");
        let flags = self
            .dict
            .get(b"Ff".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as u32;
        FieldType::from_ft_and_flags(ft, flags)
    }

    /// Get the field flags.
    pub fn flags(&self) -> FieldFlags {
        let bits = self
            .dict
            .get(b"Ff".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as u32;
        FieldFlags::from_bits_truncate(bits)
    }

    /// Get the field value as a string.
    pub fn value(&self) -> Option<String> {
        match self.dict.get(b"V".as_slice())? {
            PdfObject::Str(s) => Some(decode_text(s)),
            PdfObject::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
            PdfObject::Integer(n) => Some(n.to_string()),
            PdfObject::Real(n) => Some(n.to_string()),
            _ => None,
        }
    }

    /// Set the field value (text string).
    pub fn set_value(&mut self, value: &str) {
        self.dict.insert(
            b"V".to_vec(),
            PdfObject::Str(value.as_bytes().to_vec()),
        );
    }

    /// Set the field value as a name (for checkboxes/radio buttons: /Yes, /Off, etc.).
    pub fn set_value_name(&mut self, name: &str) {
        self.dict.insert(
            b"V".to_vec(),
            PdfObject::Name(name.as_bytes().to_vec()),
        );
    }

    /// Clear the field value.
    pub fn clear_value(&mut self) {
        self.dict.shift_remove(b"V".as_slice());
    }

    /// Get the raw dictionary (for writing back to the document).
    pub fn into_pdf_object(self) -> PdfObject {
        PdfObject::Dict(self.dict)
    }

    /// Get the default value.
    pub fn default_value(&self) -> Option<String> {
        match self.dict.get(b"DV".as_slice())? {
            PdfObject::Str(s) => Some(decode_text(s)),
            PdfObject::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
            _ => None,
        }
    }

    /// Get the maximum length for text fields.
    pub fn max_length(&self) -> Option<i64> {
        self.dict.get(b"MaxLen".as_slice()).and_then(|o| o.as_i64())
    }

    /// Get choice field options.
    pub fn options(&self) -> Vec<String> {
        match self.dict.get(b"Opt".as_slice()) {
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|item| match item {
                    PdfObject::Str(s) => Some(decode_text(s)),
                    PdfObject::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
                    PdfObject::Array(pair) if pair.len() >= 2 => {
                        pair[1].as_str().map(|s| decode_text(s))
                    }
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Get the text justification (0=left, 1=center, 2=right).
    pub fn justification(&self) -> i32 {
        self.dict
            .get(b"Q".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(0) as i32
    }

    /// Whether this field is read-only.
    pub fn is_read_only(&self) -> bool {
        self.flags().contains(FieldFlags::READ_ONLY)
    }

    /// Whether this field is required.
    pub fn is_required(&self) -> bool {
        self.flags().contains(FieldFlags::REQUIRED)
    }

    /// Whether this is a combo box (choice field with COMBO flag).
    pub fn is_combo(&self) -> bool {
        self.field_type() == FieldType::Choice && self.flags().contains(FieldFlags::COMBO)
    }

    /// Whether this is a multiline text field.
    pub fn is_multiline(&self) -> bool {
        self.field_type() == FieldType::Text && self.flags().contains(FieldFlags::MULTILINE)
    }

    /// Whether this is a password field.
    pub fn is_password(&self) -> bool {
        self.field_type() == FieldType::Text && self.flags().contains(FieldFlags::PASSWORD)
    }

    /// Get the raw dictionary.
    pub fn raw_dict(&self) -> &IndexMap<Vec<u8>, PdfObject> {
        &self.dict
    }

    /// Check if this field has child widgets (/Kids).
    pub fn has_kids(&self) -> bool {
        self.dict
            .get(b"Kids".as_slice())
            .and_then(|o| o.as_array())
            .is_some_and(|a| !a.is_empty())
    }
}

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
    fn test_field_type_parsing() {
        assert_eq!(FieldType::from_ft_and_flags(b"Tx", 0), FieldType::Text);
        assert_eq!(FieldType::from_ft_and_flags(b"Btn", 0), FieldType::CheckBox);
        assert_eq!(
            FieldType::from_ft_and_flags(b"Btn", 1 << 16),
            FieldType::Button
        );
        assert_eq!(
            FieldType::from_ft_and_flags(b"Btn", 1 << 15),
            FieldType::Radio
        );
        assert_eq!(FieldType::from_ft_and_flags(b"Ch", 0), FieldType::Choice);
        assert_eq!(
            FieldType::from_ft_and_flags(b"Sig", 0),
            FieldType::Signature
        );
    }
}
