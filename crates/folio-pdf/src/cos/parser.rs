//! PDF object parser.
//!
//! Parses PDF objects from a token stream produced by the tokenizer.
//! Handles direct objects, indirect object definitions, and object references.

use super::object::{ObjectId, PdfObject, PdfStream};
use super::tokenizer::{Token, Tokenizer};
use crate::core::{FolioError, Result};
use indexmap::IndexMap;

/// Parse a single PDF object from the tokenizer.
///
/// This may consume multiple tokens (e.g., for arrays, dicts, or references).
/// Returns None if there are no more tokens.
pub fn parse_object(tokenizer: &mut Tokenizer) -> Result<Option<PdfObject>> {
    let token = match tokenizer.next_token()? {
        Some(t) => t,
        None => return Ok(None),
    };

    match token {
        Token::Integer(n) => {
            // Could be: integer, or start of "N G R" reference, or "N G obj" definition
            let saved_pos = tokenizer.pos();
            match tokenizer.next_token()? {
                Some(Token::Integer(g)) => {
                    let _saved_pos2 = tokenizer.pos();
                    match tokenizer.next_token()? {
                        Some(Token::Keyword(ref kw)) if kw == b"R" => Ok(Some(
                            PdfObject::Reference(ObjectId::new(n as u32, g as u16)),
                        )),
                        Some(Token::Keyword(ref kw)) if kw == b"obj" => {
                            // Indirect object definition — parse the contained object
                            let obj = parse_object(tokenizer)?.unwrap_or(PdfObject::Null);
                            // Skip 'endobj'
                            skip_keyword(tokenizer, b"endobj");
                            Ok(Some(obj))
                        }
                        _ => {
                            // Not a reference or obj — put back both tokens
                            tokenizer.set_pos(saved_pos);
                            Ok(Some(PdfObject::Integer(n)))
                        }
                    }
                }
                _ => {
                    tokenizer.set_pos(saved_pos);
                    Ok(Some(PdfObject::Integer(n)))
                }
            }
        }
        Token::Real(n) => Ok(Some(PdfObject::Real(n))),
        Token::LiteralString(s) => Ok(Some(PdfObject::Str(s))),
        Token::HexString(s) => Ok(Some(PdfObject::Str(s))),
        Token::Name(n) => Ok(Some(PdfObject::Name(n))),
        Token::Keyword(ref kw) => match kw.as_slice() {
            b"true" => Ok(Some(PdfObject::Bool(true))),
            b"false" => Ok(Some(PdfObject::Bool(false))),
            b"null" => Ok(Some(PdfObject::Null)),
            _ => {
                // Unknown keyword — return as-is for caller to handle
                // (e.g., endobj, endstream, etc.)
                Ok(None)
            }
        },
        Token::ArrayBegin => parse_array(tokenizer).map(Some),
        Token::DictBegin => parse_dict_or_stream(tokenizer).map(Some),
        Token::ArrayEnd | Token::DictEnd => {
            // These are handled by the array/dict parsers
            Ok(None)
        }
    }
}

/// Parse an array (tokens between [ and ]).
fn parse_array(tokenizer: &mut Tokenizer) -> Result<PdfObject> {
    let mut items = Vec::new();

    loop {
        tokenizer.skip_whitespace_and_comments();

        if tokenizer.is_eof() {
            return Err(FolioError::Parse {
                offset: tokenizer.pos() as u64,
                message: "Unterminated array".into(),
            });
        }

        // Check for ] without consuming it via next_token
        if tokenizer.peek_byte() == Some(b']') {
            tokenizer.set_pos(tokenizer.pos() + 1);
            return Ok(PdfObject::Array(items));
        }

        match parse_object(tokenizer)? {
            Some(obj) => items.push(obj),
            None => {
                // Could be ] consumed as keyword, or end of input
                return Ok(PdfObject::Array(items));
            }
        }
    }
}

/// Parse a dictionary or stream (tokens after <<).
fn parse_dict_or_stream(tokenizer: &mut Tokenizer) -> Result<PdfObject> {
    let mut dict = IndexMap::new();

    loop {
        tokenizer.skip_whitespace_and_comments();

        if tokenizer.is_eof() {
            return Err(FolioError::Parse {
                offset: tokenizer.pos() as u64,
                message: "Unterminated dictionary".into(),
            });
        }

        // Check for >>
        if tokenizer.peek_byte() == Some(b'>') {
            let pos = tokenizer.pos();
            if pos + 1 < tokenizer.data().len() && tokenizer.data()[pos + 1] == b'>' {
                tokenizer.set_pos(pos + 2);

                // Check if followed by 'stream' keyword
                let saved_pos = tokenizer.pos();
                tokenizer.skip_whitespace_and_comments();
                let might_be_stream = tokenizer.pos();

                if might_be_stream + 6 <= tokenizer.data().len()
                    && &tokenizer.data()[might_be_stream..might_be_stream + 6] == b"stream"
                {
                    // Check the byte after "stream" to confirm it's the keyword
                    let after = tokenizer.data().get(might_be_stream + 6).copied();
                    if after == Some(b'\n') || after == Some(b'\r') {
                        return parse_stream(tokenizer, dict, might_be_stream);
                    }
                }

                // Not a stream — restore position
                tokenizer.set_pos(saved_pos);
                return Ok(PdfObject::Dict(dict));
            }
        }

        // Read key (must be a Name)
        let key = match tokenizer.next_token()? {
            Some(Token::Name(n)) => n,
            Some(Token::DictEnd) => return Ok(PdfObject::Dict(dict)),
            Some(other) => {
                return Err(FolioError::Parse {
                    offset: tokenizer.pos() as u64,
                    message: format!("Expected name key in dict, got {:?}", other),
                });
            }
            None => return Ok(PdfObject::Dict(dict)),
        };

        // Read value
        let value = parse_object(tokenizer)?.unwrap_or(PdfObject::Null);
        dict.insert(key, value);
    }
}

/// Parse stream data after dict and 'stream' keyword.
fn parse_stream(
    tokenizer: &mut Tokenizer,
    dict: IndexMap<Vec<u8>, PdfObject>,
    stream_keyword_pos: usize,
) -> Result<PdfObject> {
    // Position past 'stream'
    let mut pos = stream_keyword_pos + 6;

    // Skip the EOL after 'stream' (required: either \r\n or \n)
    if pos < tokenizer.data().len() && tokenizer.data()[pos] == b'\r' {
        pos += 1;
    }
    if pos < tokenizer.data().len() && tokenizer.data()[pos] == b'\n' {
        pos += 1;
    }

    // Get stream length from dictionary
    let length = dict
        .get(b"Length".as_slice())
        .and_then(|obj| obj.as_i64())
        .unwrap_or(0) as usize;

    let end_pos = (pos + length).min(tokenizer.data().len());
    let data = tokenizer.data()[pos..end_pos].to_vec();

    // Skip past the data + 'endstream'
    let mut search_pos = end_pos;
    // Skip whitespace before endstream
    while search_pos < tokenizer.data().len()
        && (tokenizer.data()[search_pos] == b'\r'
            || tokenizer.data()[search_pos] == b'\n'
            || tokenizer.data()[search_pos] == b' ')
    {
        search_pos += 1;
    }
    // Skip 'endstream' keyword
    if search_pos + 9 <= tokenizer.data().len()
        && &tokenizer.data()[search_pos..search_pos + 9] == b"endstream"
    {
        search_pos += 9;
    }

    tokenizer.set_pos(search_pos);

    Ok(PdfObject::Stream(PdfStream {
        dict,
        data,
        decoded: false,
    }))
}

/// Skip an expected keyword (non-fatal if not found).
fn skip_keyword(tokenizer: &mut Tokenizer, expected: &[u8]) {
    let saved = tokenizer.pos();
    tokenizer.skip_whitespace_and_comments();
    if let Ok(Some(Token::Keyword(kw))) = tokenizer.next_token() {
        if kw == expected {
            return;
        }
    }
    tokenizer.set_pos(saved);
}

/// Parse an indirect object at a given byte offset.
/// Returns (ObjectId, PdfObject).
pub fn parse_indirect_object_at(data: &[u8], offset: usize) -> Result<(ObjectId, PdfObject)> {
    let mut tokenizer = Tokenizer::new_at(data, offset);

    let obj_num = match tokenizer.next_token()? {
        Some(Token::Integer(n)) => n as u32,
        other => {
            return Err(FolioError::Parse {
                offset: offset as u64,
                message: format!("Expected object number, got {:?}", other),
            });
        }
    };

    let gen_num = match tokenizer.next_token()? {
        Some(Token::Integer(n)) => n as u16,
        other => {
            return Err(FolioError::Parse {
                offset: offset as u64,
                message: format!("Expected generation number, got {:?}", other),
            });
        }
    };

    match tokenizer.next_token()? {
        Some(Token::Keyword(ref kw)) if kw == b"obj" => {}
        other => {
            return Err(FolioError::Parse {
                offset: offset as u64,
                message: format!("Expected 'obj' keyword, got {:?}", other),
            });
        }
    }

    let obj = parse_object(&mut tokenizer)?.unwrap_or(PdfObject::Null);

    // Skip 'endobj'
    skip_keyword(&mut tokenizer, b"endobj");

    Ok((ObjectId::new(obj_num, gen_num), obj))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> PdfObject {
        let mut t = Tokenizer::new(input);
        parse_object(&mut t).unwrap().unwrap()
    }

    #[test]
    fn test_primitives() {
        assert_eq!(parse(b"42"), PdfObject::Integer(42));
        assert_eq!(parse(b"3.14"), PdfObject::Real(3.14));
        assert_eq!(parse(b"true"), PdfObject::Bool(true));
        assert_eq!(parse(b"false"), PdfObject::Bool(false));
        assert_eq!(parse(b"null"), PdfObject::Null);
    }

    #[test]
    fn test_name() {
        assert_eq!(parse(b"/Type"), PdfObject::Name(b"Type".to_vec()));
    }

    #[test]
    fn test_string() {
        assert_eq!(parse(b"(Hello)"), PdfObject::Str(b"Hello".to_vec()));
        assert_eq!(parse(b"<48656C6C6F>"), PdfObject::Str(b"Hello".to_vec()));
    }

    #[test]
    fn test_array() {
        let obj = parse(b"[1 2 3]");
        let arr = obj.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[2].as_i64(), Some(3));
    }

    #[test]
    fn test_dict() {
        let obj = parse(b"<< /Type /Page /Count 5 >>");
        let dict = obj.as_dict().unwrap();
        assert_eq!(dict.len(), 2);
        assert_eq!(
            dict.get(b"Type".as_slice()).unwrap().as_name(),
            Some(b"Page".as_slice())
        );
        assert_eq!(dict.get(b"Count".as_slice()).unwrap().as_i64(), Some(5));
    }

    #[test]
    fn test_reference() {
        let obj = parse(b"3 0 R");
        assert_eq!(obj.as_reference(), Some(ObjectId::new(3, 0)));
    }

    #[test]
    fn test_nested() {
        let obj = parse(b"<< /Kids [1 0 R 2 0 R] /Count 2 >>");
        let kids = obj.dict_get(b"Kids").unwrap().as_array().unwrap();
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].as_reference(), Some(ObjectId::new(1, 0)));
    }

    #[test]
    fn test_indirect_object() {
        let input = b"1 0 obj\n<< /Type /Catalog >>\nendobj";
        let (id, obj) = parse_indirect_object_at(input, 0).unwrap();
        assert_eq!(id, ObjectId::new(1, 0));
        assert_eq!(obj.dict_get_name(b"Type"), Some(b"Catalog".as_slice()));
    }

    #[test]
    fn test_stream() {
        let input = b"<< /Length 5 >>\nstream\nHello\nendstream";
        let obj = parse(input);
        let stream = obj.as_stream().unwrap();
        assert_eq!(&stream.data, b"Hello");
        assert_eq!(
            stream.dict.get(b"Length".as_slice()).unwrap().as_i64(),
            Some(5)
        );
    }
}
