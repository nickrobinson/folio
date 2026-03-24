//! ASCIIHexDecode / ASCIIHexEncode — hex encoding for PDF streams.

use folio_core::{FolioError, Result};

/// Decode ASCIIHex-encoded data.
///
/// Input consists of hex digit pairs, optionally separated by whitespace.
/// The `>` character marks end-of-data.
pub fn asciihex_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(data.len() / 2);
    let mut high_nibble: Option<u8> = None;

    for &byte in data {
        match byte {
            b'>' => break,
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' => continue,
            _ => {
                let nibble = hex_digit(byte).ok_or_else(|| FolioError::Parse {
                    offset: 0,
                    message: format!("Invalid hex digit: 0x{:02x}", byte),
                })?;
                match high_nibble {
                    None => high_nibble = Some(nibble),
                    Some(high) => {
                        output.push((high << 4) | nibble);
                        high_nibble = None;
                    }
                }
            }
        }
    }

    // If odd number of hex digits, treat the last one as if followed by 0
    if let Some(high) = high_nibble {
        output.push(high << 4);
    }

    Ok(output)
}

/// Encode data using ASCIIHex.
pub fn asciihex_encode(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(data.len() * 2 + 1);
    for &byte in data {
        output.push(HEX_CHARS[(byte >> 4) as usize]);
        output.push(HEX_CHARS[(byte & 0x0f) as usize]);
    }
    output.push(b'>');
    Ok(output)
}

const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

fn hex_digit(byte: u8) -> Option<u8> {
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

    #[test]
    fn test_roundtrip() {
        let original = b"\x00\x01\xff\xab\xcd";
        let encoded = asciihex_encode(original).unwrap();
        let decoded = asciihex_decode(&encoded).unwrap();
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_decode_with_whitespace() {
        let decoded = asciihex_decode(b"48 65 6C 6C 6F>").unwrap();
        assert_eq!(&decoded, b"Hello");
    }

    #[test]
    fn test_decode_odd_nibble() {
        // Odd number of hex digits: last nibble padded with 0
        let decoded = asciihex_decode(b"ABC>").unwrap();
        assert_eq!(decoded, vec![0xAB, 0xC0]);
    }

    #[test]
    fn test_empty() {
        let decoded = asciihex_decode(b">").unwrap();
        assert!(decoded.is_empty());
    }
}
