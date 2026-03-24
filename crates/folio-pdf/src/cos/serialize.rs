//! PDF object serialization.
//!
//! Converts PdfObject values back into PDF byte sequences.

use super::object::{ObjectId, PdfObject, PdfStream};
use crate::core::Result;

/// Serialize a PDF object to bytes.
pub fn serialize_object(obj: &PdfObject) -> Vec<u8> {
    let mut buf = Vec::new();
    write_object(obj, &mut buf);
    buf
}

/// Serialize an indirect object definition to bytes.
pub fn serialize_indirect_object(id: ObjectId, obj: &PdfObject) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("{} {} obj\n", id.num, id.gen_num).as_bytes());
    write_object(obj, &mut buf);
    buf.push(b'\n');
    buf.extend_from_slice(b"endobj\n");
    buf
}

/// Write a PDF object into a byte buffer.
fn write_object(obj: &PdfObject, buf: &mut Vec<u8>) {
    match obj {
        PdfObject::Null => buf.extend_from_slice(b"null"),
        PdfObject::Bool(true) => buf.extend_from_slice(b"true"),
        PdfObject::Bool(false) => buf.extend_from_slice(b"false"),
        PdfObject::Integer(n) => buf.extend_from_slice(n.to_string().as_bytes()),
        PdfObject::Real(n) => {
            // Format with enough precision but no trailing zeros
            let s = format!("{}", n);
            buf.extend_from_slice(s.as_bytes());
        }
        PdfObject::Name(name) => {
            buf.push(b'/');
            for &byte in name {
                if byte == b'#'
                    || byte <= b' '
                    || byte >= 127
                    || matches!(
                        byte,
                        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
                    )
                {
                    buf.push(b'#');
                    buf.push(HEX_UPPER[(byte >> 4) as usize]);
                    buf.push(HEX_UPPER[(byte & 0xf) as usize]);
                } else {
                    buf.push(byte);
                }
            }
        }
        PdfObject::Str(s) => write_literal_string(s, buf),
        PdfObject::Array(items) => {
            buf.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    buf.push(b' ');
                }
                write_object(item, buf);
            }
            buf.push(b']');
        }
        PdfObject::Dict(dict) => write_dict(dict, buf),
        PdfObject::Reference(id) => {
            buf.extend_from_slice(format!("{} {} R", id.num, id.gen_num).as_bytes());
        }
        PdfObject::Stream(stream) => write_stream(stream, buf),
    }
}

fn write_dict(dict: &indexmap::IndexMap<Vec<u8>, PdfObject>, buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"<<");
    for (key, value) in dict {
        buf.push(b'/');
        buf.extend_from_slice(key);
        buf.push(b' ');
        write_object(value, buf);
        buf.push(b' ');
    }
    buf.extend_from_slice(b">>");
}

fn write_stream(stream: &PdfStream, buf: &mut Vec<u8>) {
    // Update Length in dict
    let mut dict = stream.dict.clone();
    dict.insert(
        b"Length".to_vec(),
        PdfObject::Integer(stream.data.len() as i64),
    );

    write_dict(&dict, buf);
    buf.extend_from_slice(b"\nstream\n");
    buf.extend_from_slice(&stream.data);
    buf.extend_from_slice(b"\nendstream");
}

fn write_literal_string(s: &[u8], buf: &mut Vec<u8>) {
    buf.push(b'(');
    for &byte in s {
        match byte {
            b'\\' => buf.extend_from_slice(b"\\\\"),
            b'(' => buf.extend_from_slice(b"\\("),
            b')' => buf.extend_from_slice(b"\\)"),
            b'\n' => buf.extend_from_slice(b"\\n"),
            b'\r' => buf.extend_from_slice(b"\\r"),
            b'\t' => buf.extend_from_slice(b"\\t"),
            0x08 => buf.extend_from_slice(b"\\b"),
            0x0c => buf.extend_from_slice(b"\\f"),
            _ if byte < 32 || byte > 126 => {
                buf.push(b'\\');
                buf.push(b'0' + (byte >> 6));
                buf.push(b'0' + ((byte >> 3) & 7));
                buf.push(b'0' + (byte & 7));
            }
            _ => buf.push(byte),
        }
    }
    buf.push(b')');
}

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Serialize a complete PDF document from its components.
pub fn serialize_pdf(
    objects: &[(ObjectId, PdfObject)],
    trailer_dict: &indexmap::IndexMap<Vec<u8>, PdfObject>,
) -> Result<Vec<u8>> {
    let mut buf = Vec::new();

    // Header
    buf.extend_from_slice(b"%PDF-1.7\n");
    // Binary comment to signal binary content
    buf.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");

    // Write objects and record offsets
    let mut offsets: Vec<(ObjectId, usize)> = Vec::new();

    for (id, obj) in objects {
        offsets.push((*id, buf.len()));
        buf.extend_from_slice(&serialize_indirect_object(*id, obj));
    }

    // Write xref table
    let xref_offset = buf.len();
    buf.extend_from_slice(b"xref\n");

    let max_obj = offsets.iter().map(|(id, _)| id.num).max().unwrap_or(0);
    buf.extend_from_slice(format!("0 {}\n", max_obj + 1).as_bytes());

    // Entry for object 0 (free head)
    buf.extend_from_slice(b"0000000000 65535 f \n");

    for obj_num in 1..=max_obj {
        if let Some((_, offset)) = offsets.iter().find(|(id, _)| id.num == obj_num) {
            buf.extend_from_slice(format!("{:010} {:05} n \n", offset, 0).as_bytes());
        } else {
            buf.extend_from_slice(b"0000000000 00000 f \n");
        }
    }

    // Write trailer
    buf.extend_from_slice(b"trailer\n");
    let mut trailer = trailer_dict.clone();
    trailer.insert(b"Size".to_vec(), PdfObject::Integer((max_obj + 1) as i64));
    write_dict(&trailer, &mut buf);
    buf.push(b'\n');

    // Write startxref
    buf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_primitives() {
        assert_eq!(serialize_object(&PdfObject::Null), b"null");
        assert_eq!(serialize_object(&PdfObject::Bool(true)), b"true");
        assert_eq!(serialize_object(&PdfObject::Integer(42)), b"42");
        assert_eq!(
            serialize_object(&PdfObject::Name(b"Type".to_vec())),
            b"/Type"
        );
    }

    #[test]
    fn test_serialize_string() {
        assert_eq!(
            serialize_object(&PdfObject::Str(b"Hello".to_vec())),
            b"(Hello)"
        );
        assert_eq!(
            serialize_object(&PdfObject::Str(b"Hello\nWorld".to_vec())),
            b"(Hello\\nWorld)"
        );
    }

    #[test]
    fn test_serialize_reference() {
        assert_eq!(
            serialize_object(&PdfObject::Reference(ObjectId::new(3, 0))),
            b"3 0 R"
        );
    }

    #[test]
    fn test_roundtrip() {
        use crate::cos::parser;
        use crate::cos::tokenizer::Tokenizer;

        let original = PdfObject::Integer(42);
        let bytes = serialize_object(&original);
        let mut tok = Tokenizer::new(&bytes);
        let parsed = parser::parse_object(&mut tok).unwrap().unwrap();
        assert_eq!(original, parsed);
    }
}
