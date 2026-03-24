//! PDF tokenizer.
//!
//! Breaks a raw byte stream into a sequence of PDF tokens.
//! Handles all PDF lexical conventions per ISO 32000-2:2020 §7.2.

use crate::core::{FolioError, Result};

/// A PDF token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// An integer number.
    Integer(i64),
    /// A real (floating-point) number.
    Real(f64),
    /// A literal string (between parentheses), with escape sequences resolved.
    LiteralString(Vec<u8>),
    /// A hexadecimal string (between angle brackets).
    HexString(Vec<u8>),
    /// A name (after the leading /).
    Name(Vec<u8>),
    /// A keyword (true, false, null, obj, endobj, stream, endstream, xref, trailer, startxref, R, etc.)
    Keyword(Vec<u8>),
    /// Start of array: [
    ArrayBegin,
    /// End of array: ]
    ArrayEnd,
    /// Start of dictionary: <<
    DictBegin,
    /// End of dictionary: >>
    DictEnd,
}

/// PDF tokenizer operating on a byte slice.
pub struct Tokenizer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Create a tokenizer starting at a specific offset.
    pub fn new_at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    /// Current byte position in the data.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Set position directly.
    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Peek at the current byte without consuming it.
    pub fn peek_byte(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Check if we've reached the end of data.
    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Get a slice of the underlying data.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Read the next token, skipping whitespace and comments.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace_and_comments();

        if self.is_eof() {
            return Ok(None);
        }

        let byte = self.data[self.pos];

        match byte {
            b'(' => self.read_literal_string().map(Some),
            b'<' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                    self.pos += 2;
                    Ok(Some(Token::DictBegin))
                } else {
                    self.read_hex_string().map(Some)
                }
            }
            b'>' => {
                if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                    self.pos += 2;
                    Ok(Some(Token::DictEnd))
                } else {
                    self.pos += 1;
                    Err(FolioError::Parse {
                        offset: self.pos as u64 - 1,
                        message: "Unexpected '>'".into(),
                    })
                }
            }
            b'[' => {
                self.pos += 1;
                Ok(Some(Token::ArrayBegin))
            }
            b']' => {
                self.pos += 1;
                Ok(Some(Token::ArrayEnd))
            }
            b'/' => self.read_name().map(Some),
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number().map(Some),
            _ => self.read_keyword().map(Some),
        }
    }

    /// Skip whitespace and comments.
    pub fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            if is_whitespace(byte) {
                self.pos += 1;
            } else if byte == b'%' {
                // Skip comment until end of line
                self.pos += 1;
                while self.pos < self.data.len()
                    && self.data[self.pos] != b'\n'
                    && self.data[self.pos] != b'\r'
                {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Skip whitespace only (no comments).
    pub fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && is_whitespace(self.data[self.pos]) {
            self.pos += 1;
        }
    }

    /// Read a literal string (between parentheses with nesting support).
    fn read_literal_string(&mut self) -> Result<Token> {
        debug_assert_eq!(self.data[self.pos], b'(');
        self.pos += 1; // skip opening (

        let mut result = Vec::new();
        let mut depth = 1u32;

        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;

            match byte {
                b'(' => {
                    depth += 1;
                    result.push(b'(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(Token::LiteralString(result));
                    }
                    result.push(b')');
                }
                b'\\' => {
                    if self.pos >= self.data.len() {
                        result.push(b'\\');
                        break;
                    }
                    let escaped = self.data[self.pos];
                    self.pos += 1;
                    match escaped {
                        b'n' => result.push(b'\n'),
                        b'r' => result.push(b'\r'),
                        b't' => result.push(b'\t'),
                        b'b' => result.push(0x08),
                        b'f' => result.push(0x0C),
                        b'(' => result.push(b'('),
                        b')' => result.push(b')'),
                        b'\\' => result.push(b'\\'),
                        b'\r' => {
                            // Line continuation: \<CR> or \<CR><LF>
                            if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {
                            // Line continuation: \<LF>
                        }
                        b'0'..=b'7' => {
                            // Octal character code (1-3 digits)
                            let mut octal = (escaped - b'0') as u32;
                            for _ in 0..2 {
                                if self.pos < self.data.len()
                                    && self.data[self.pos] >= b'0'
                                    && self.data[self.pos] <= b'7'
                                {
                                    octal = octal * 8 + (self.data[self.pos] - b'0') as u32;
                                    self.pos += 1;
                                } else {
                                    break;
                                }
                            }
                            result.push((octal & 0xFF) as u8);
                        }
                        _ => {
                            // Unknown escape — ignore the backslash per spec
                            result.push(escaped);
                        }
                    }
                }
                _ => result.push(byte),
            }
        }

        Err(FolioError::Parse {
            offset: self.pos as u64,
            message: "Unterminated literal string".into(),
        })
    }

    /// Read a hexadecimal string (between < and >).
    fn read_hex_string(&mut self) -> Result<Token> {
        debug_assert_eq!(self.data[self.pos], b'<');
        self.pos += 1; // skip opening <

        let mut hex_bytes = Vec::new();

        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            self.pos += 1;

            match byte {
                b'>' => {
                    // Decode hex pairs
                    let mut result = Vec::with_capacity(hex_bytes.len() / 2);
                    let mut i = 0;
                    while i < hex_bytes.len() {
                        let high = hex_bytes[i];
                        let low = if i + 1 < hex_bytes.len() {
                            hex_bytes[i + 1]
                        } else {
                            0 // Odd number of digits — pad with 0
                        };
                        result.push((high << 4) | low);
                        i += 2;
                    }
                    return Ok(Token::HexString(result));
                }
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x00' => continue,
                b'0'..=b'9' => hex_bytes.push(byte - b'0'),
                b'a'..=b'f' => hex_bytes.push(byte - b'a' + 10),
                b'A'..=b'F' => hex_bytes.push(byte - b'A' + 10),
                _ => {
                    return Err(FolioError::Parse {
                        offset: self.pos as u64 - 1,
                        message: format!("Invalid hex digit: 0x{:02x}", byte),
                    });
                }
            }
        }

        Err(FolioError::Parse {
            offset: self.pos as u64,
            message: "Unterminated hex string".into(),
        })
    }

    /// Read a name object (starts with /).
    fn read_name(&mut self) -> Result<Token> {
        debug_assert_eq!(self.data[self.pos], b'/');
        self.pos += 1; // skip /

        let mut name = Vec::new();

        while self.pos < self.data.len() {
            let byte = self.data[self.pos];

            if is_whitespace(byte) || is_delimiter(byte) {
                break;
            }

            self.pos += 1;

            if byte == b'#' && self.pos + 1 < self.data.len() {
                // Hex-encoded character: #XX
                let h1 = hex_val(self.data[self.pos]);
                let h2 = hex_val(self.data[self.pos + 1]);
                if let (Some(high), Some(low)) = (h1, h2) {
                    name.push((high << 4) | low);
                    self.pos += 2;
                } else {
                    name.push(b'#');
                }
            } else {
                name.push(byte);
            }
        }

        Ok(Token::Name(name))
    }

    /// Read a number (integer or real).
    fn read_number(&mut self) -> Result<Token> {
        let start = self.pos;
        let mut has_dot = false;

        // Optional sign
        if self.pos < self.data.len()
            && (self.data[self.pos] == b'+' || self.data[self.pos] == b'-')
        {
            self.pos += 1;
        }

        // Digits and optional dot
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            match byte {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !has_dot => {
                    has_dot = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }

        let num_str =
            std::str::from_utf8(&self.data[start..self.pos]).map_err(|_| FolioError::Parse {
                offset: start as u64,
                message: "Invalid number encoding".into(),
            })?;

        if has_dot {
            let val: f64 = num_str.parse().map_err(|_| FolioError::Parse {
                offset: start as u64,
                message: format!("Invalid real number: '{}'", num_str),
            })?;
            Ok(Token::Real(val))
        } else {
            // Try integer first, fall back to real for very large numbers
            match num_str.parse::<i64>() {
                Ok(val) => Ok(Token::Integer(val)),
                Err(_) => {
                    let val: f64 = num_str.parse().map_err(|_| FolioError::Parse {
                        offset: start as u64,
                        message: format!("Invalid number: '{}'", num_str),
                    })?;
                    Ok(Token::Real(val))
                }
            }
        }
    }

    /// Read a keyword (alphabetic sequence).
    fn read_keyword(&mut self) -> Result<Token> {
        let start = self.pos;
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            if is_whitespace(byte) || is_delimiter(byte) {
                break;
            }
            self.pos += 1;
        }

        if self.pos == start {
            return Err(FolioError::Parse {
                offset: start as u64,
                message: format!(
                    "Unexpected byte: 0x{:02x}",
                    self.data.get(start).copied().unwrap_or(0)
                ),
            });
        }

        Ok(Token::Keyword(self.data[start..self.pos].to_vec()))
    }
}

/// Check if a byte is PDF whitespace.
pub fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b'\x00' | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Check if a byte is a PDF delimiter.
pub fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// Convert a hex digit byte to its value.
fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &[u8]) -> Vec<Token> {
        let mut t = Tokenizer::new(input);
        let mut tokens = Vec::new();
        while let Ok(Some(tok)) = t.next_token() {
            tokens.push(tok);
        }
        tokens
    }

    #[test]
    fn test_integer() {
        assert_eq!(tokenize(b"42"), vec![Token::Integer(42)]);
        assert_eq!(tokenize(b"-17"), vec![Token::Integer(-17)]);
        assert_eq!(tokenize(b"+5"), vec![Token::Integer(5)]);
        assert_eq!(tokenize(b"0"), vec![Token::Integer(0)]);
    }

    #[test]
    fn test_real() {
        assert_eq!(tokenize(b"3.14"), vec![Token::Real(3.14)]);
        assert_eq!(tokenize(b"-0.5"), vec![Token::Real(-0.5)]);
        assert_eq!(tokenize(b".25"), vec![Token::Real(0.25)]);
    }

    #[test]
    fn test_name() {
        assert_eq!(tokenize(b"/Type"), vec![Token::Name(b"Type".to_vec())]);
        assert_eq!(tokenize(b"/A#42"), vec![Token::Name(b"AB".to_vec())]);
    }

    #[test]
    fn test_literal_string() {
        assert_eq!(
            tokenize(b"(Hello)"),
            vec![Token::LiteralString(b"Hello".to_vec())]
        );
        assert_eq!(
            tokenize(b"(Hello\\nWorld)"),
            vec![Token::LiteralString(b"Hello\nWorld".to_vec())]
        );
        // Nested parens
        assert_eq!(
            tokenize(b"(Hello (World))"),
            vec![Token::LiteralString(b"Hello (World)".to_vec())]
        );
    }

    #[test]
    fn test_hex_string() {
        assert_eq!(
            tokenize(b"<48656C6C6F>"),
            vec![Token::HexString(b"Hello".to_vec())]
        );
        assert_eq!(
            tokenize(b"<48 65 6C>"),
            vec![Token::HexString(b"Hel".to_vec())]
        );
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize(b"true false null"),
            vec![
                Token::Keyword(b"true".to_vec()),
                Token::Keyword(b"false".to_vec()),
                Token::Keyword(b"null".to_vec()),
            ]
        );
    }

    #[test]
    fn test_delimiters() {
        assert_eq!(
            tokenize(b"[1 2]"),
            vec![
                Token::ArrayBegin,
                Token::Integer(1),
                Token::Integer(2),
                Token::ArrayEnd,
            ]
        );
        assert_eq!(
            tokenize(b"<< /Key /Value >>"),
            vec![
                Token::DictBegin,
                Token::Name(b"Key".to_vec()),
                Token::Name(b"Value".to_vec()),
                Token::DictEnd,
            ]
        );
    }

    #[test]
    fn test_comments() {
        assert_eq!(
            tokenize(b"42 % this is a comment\n17"),
            vec![Token::Integer(42), Token::Integer(17)]
        );
    }

    #[test]
    fn test_mixed() {
        let tokens = tokenize(b"/Type /Page /MediaBox [0 0 612 792]");
        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0], Token::Name(b"Type".to_vec()));
        assert_eq!(tokens[1], Token::Name(b"Page".to_vec()));
        assert_eq!(tokens[2], Token::Name(b"MediaBox".to_vec()));
        assert_eq!(tokens[3], Token::ArrayBegin);
        assert_eq!(tokens[8], Token::ArrayEnd);
    }

    #[test]
    fn test_octal_escape() {
        assert_eq!(
            tokenize(b"(\\110\\145\\154\\154\\157)"),
            vec![Token::LiteralString(b"Hello".to_vec())]
        );
    }
}
