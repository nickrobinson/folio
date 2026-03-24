//! PDF object types.
//!
//! PDF has 8 basic object types: null, boolean, integer, real, name, string,
//! array, dictionary, and stream. Objects can be "indirect" (referenced by
//! object number and generation number).

use indexmap::IndexMap;
use std::fmt;

/// A unique identifier for an indirect object (object number + generation number).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId {
    pub num: u32,
    pub gen_num: u16,
}

impl ObjectId {
    pub fn new(num: u32, gen_num: u16) -> Self {
        Self { num, gen_num }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.num, self.gen_num)
    }
}

/// A PDF object.
///
/// This enum represents all 8 PDF object types plus an indirect reference.
/// Stream objects are represented separately as they contain both a dictionary
/// and binary data.
#[derive(Debug, Clone)]
pub enum PdfObject {
    /// The null object.
    Null,
    /// A boolean value.
    Bool(bool),
    /// An integer value (PDF integers are at least 32-bit).
    Integer(i64),
    /// A real (floating-point) value.
    Real(f64),
    /// A name object (e.g., /Type, /Page). Stored without the leading '/'.
    Name(Vec<u8>),
    /// A string object (literal or hex string).
    Str(Vec<u8>),
    /// An array of objects.
    Array(Vec<PdfObject>),
    /// A dictionary mapping name keys to object values.
    Dict(IndexMap<Vec<u8>, PdfObject>),
    /// A reference to an indirect object.
    Reference(ObjectId),
    /// A stream object (dictionary + binary data).
    /// The data is stored in decoded (uncompressed) form when possible.
    Stream(PdfStream),
}

/// A PDF stream object: dictionary + binary data.
#[derive(Debug, Clone)]
pub struct PdfStream {
    /// The stream dictionary (contains /Length, /Filter, etc.)
    pub dict: IndexMap<Vec<u8>, PdfObject>,
    /// The raw (possibly still encoded) stream data.
    pub data: Vec<u8>,
    /// Whether `data` has been decoded (filters applied).
    pub decoded: bool,
}

impl PdfObject {
    // --- Type checking ---

    pub fn is_null(&self) -> bool {
        matches!(self, PdfObject::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, PdfObject::Bool(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, PdfObject::Integer(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, PdfObject::Integer(_) | PdfObject::Real(_))
    }

    pub fn is_name(&self) -> bool {
        matches!(self, PdfObject::Name(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, PdfObject::Str(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, PdfObject::Array(_))
    }

    pub fn is_dict(&self) -> bool {
        matches!(self, PdfObject::Dict(_))
    }

    pub fn is_stream(&self) -> bool {
        matches!(self, PdfObject::Stream(_))
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, PdfObject::Reference(_))
    }

    // --- Value extraction ---

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(n) => Some(*n),
            PdfObject::Real(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PdfObject::Integer(n) => Some(*n as f64),
            PdfObject::Real(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&[u8]> {
        match self {
            PdfObject::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Get name as a UTF-8 string (lossy).
    pub fn as_name_str(&self) -> Option<String> {
        self.as_name()
            .map(|n| String::from_utf8_lossy(n).into_owned())
    }

    pub fn as_str(&self) -> Option<&[u8]> {
        match self {
            PdfObject::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<PdfObject>> {
        match self {
            PdfObject::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&IndexMap<Vec<u8>, PdfObject>> {
        match self {
            PdfObject::Dict(d) => Some(d),
            PdfObject::Stream(s) => Some(&s.dict),
            _ => None,
        }
    }

    pub fn as_dict_mut(&mut self) -> Option<&mut IndexMap<Vec<u8>, PdfObject>> {
        match self {
            PdfObject::Dict(d) => Some(d),
            PdfObject::Stream(s) => Some(&mut s.dict),
            _ => None,
        }
    }

    pub fn as_reference(&self) -> Option<ObjectId> {
        match self {
            PdfObject::Reference(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<&PdfStream> {
        match self {
            PdfObject::Stream(s) => Some(s),
            _ => None,
        }
    }

    // --- Dictionary helpers ---

    /// Look up a key in a dictionary (or stream dictionary).
    pub fn dict_get(&self, key: &[u8]) -> Option<&PdfObject> {
        self.as_dict()?.get(key)
    }

    /// Look up a key and get it as an integer.
    pub fn dict_get_i64(&self, key: &[u8]) -> Option<i64> {
        self.dict_get(key)?.as_i64()
    }

    /// Look up a key and get it as a float.
    pub fn dict_get_f64(&self, key: &[u8]) -> Option<f64> {
        self.dict_get(key)?.as_f64()
    }

    /// Look up a key and get it as a name.
    pub fn dict_get_name(&self, key: &[u8]) -> Option<&[u8]> {
        self.dict_get(key)?.as_name()
    }

    /// Look up a key and get it as a name string.
    pub fn dict_get_name_str(&self, key: &[u8]) -> Option<String> {
        self.dict_get(key)?.as_name_str()
    }

    /// Look up a key and get it as a boolean.
    pub fn dict_get_bool(&self, key: &[u8]) -> Option<bool> {
        self.dict_get(key)?.as_bool()
    }
}

impl fmt::Display for PdfObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfObject::Null => write!(f, "null"),
            PdfObject::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            PdfObject::Integer(n) => write!(f, "{}", n),
            PdfObject::Real(n) => write!(f, "{}", n),
            PdfObject::Name(n) => write!(f, "/{}", String::from_utf8_lossy(n)),
            PdfObject::Str(s) => write!(f, "({})", String::from_utf8_lossy(s)),
            PdfObject::Array(a) => {
                write!(f, "[")?;
                for (i, obj) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", obj)?;
                }
                write!(f, "]")
            }
            PdfObject::Dict(d) => {
                write!(f, "<< ")?;
                for (k, v) in d {
                    write!(f, "/{} {} ", String::from_utf8_lossy(k), v)?;
                }
                write!(f, ">>")
            }
            PdfObject::Reference(id) => write!(f, "{}", id),
            PdfObject::Stream(s) => {
                write!(f, "<< ")?;
                for (k, v) in &s.dict {
                    write!(f, "/{} {} ", String::from_utf8_lossy(k), v)?;
                }
                write!(f, ">> stream[{}bytes]", s.data.len())
            }
        }
    }
}

impl Default for PdfObject {
    fn default() -> Self {
        PdfObject::Null
    }
}

impl PartialEq for PdfObject {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PdfObject::Null, PdfObject::Null) => true,
            (PdfObject::Bool(a), PdfObject::Bool(b)) => a == b,
            (PdfObject::Integer(a), PdfObject::Integer(b)) => a == b,
            (PdfObject::Real(a), PdfObject::Real(b)) => a == b,
            (PdfObject::Name(a), PdfObject::Name(b)) => a == b,
            (PdfObject::Str(a), PdfObject::Str(b)) => a == b,
            (PdfObject::Array(a), PdfObject::Array(b)) => a == b,
            (PdfObject::Reference(a), PdfObject::Reference(b)) => a == b,
            // Dicts and streams compare by keys/values
            (PdfObject::Dict(a), PdfObject::Dict(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_types() {
        assert!(PdfObject::Null.is_null());
        assert!(PdfObject::Bool(true).is_bool());
        assert!(PdfObject::Integer(42).is_integer());
        assert!(PdfObject::Integer(42).is_number());
        assert!(PdfObject::Real(3.14).is_number());
        assert!(PdfObject::Name(b"Type".to_vec()).is_name());
        assert!(PdfObject::Str(b"hello".to_vec()).is_string());
        assert!(PdfObject::Array(vec![]).is_array());
        assert!(PdfObject::Dict(IndexMap::new()).is_dict());
        assert!(PdfObject::Reference(ObjectId::new(1, 0)).is_reference());
    }

    #[test]
    fn test_value_extraction() {
        assert_eq!(PdfObject::Bool(true).as_bool(), Some(true));
        assert_eq!(PdfObject::Integer(42).as_i64(), Some(42));
        assert_eq!(PdfObject::Integer(42).as_f64(), Some(42.0));
        assert_eq!(PdfObject::Real(3.14).as_f64(), Some(3.14));
        assert_eq!(
            PdfObject::Name(b"Type".to_vec()).as_name(),
            Some(b"Type".as_slice())
        );
    }

    #[test]
    fn test_dict_helpers() {
        let mut dict = IndexMap::new();
        dict.insert(b"Type".to_vec(), PdfObject::Name(b"Page".to_vec()));
        dict.insert(b"Count".to_vec(), PdfObject::Integer(5));
        let obj = PdfObject::Dict(dict);

        assert_eq!(obj.dict_get_name_str(b"Type"), Some("Page".to_string()));
        assert_eq!(obj.dict_get_i64(b"Count"), Some(5));
        assert_eq!(obj.dict_get(b"Missing"), None);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", PdfObject::Null), "null");
        assert_eq!(format!("{}", PdfObject::Bool(true)), "true");
        assert_eq!(format!("{}", PdfObject::Integer(42)), "42");
        assert_eq!(format!("{}", PdfObject::Name(b"Type".to_vec())), "/Type");
        assert_eq!(
            format!("{}", PdfObject::Reference(ObjectId::new(3, 0))),
            "3 0 R"
        );
    }
}
