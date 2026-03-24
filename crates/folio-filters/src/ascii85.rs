//! ASCII85Decode / ASCII85Encode — Base-85 encoding used in PDF streams.

use folio_core::{FolioError, Result};

/// Decode ASCII85-encoded data.
///
/// The input may optionally start with `<~` and end with `~>`.
pub fn ascii85_decode(data: &[u8]) -> Result<Vec<u8>> {
    let data = if data.starts_with(b"<~") {
        &data[2..]
    } else {
        data
    };

    let mut output = Vec::with_capacity(data.len() * 4 / 5);
    let mut group: u32 = 0;
    let mut count = 0;

    for &byte in data {
        match byte {
            // EOD marker
            b'~' => break,
            // Skip whitespace
            b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' => continue,
            // Special 'z' = four zero bytes
            b'z' => {
                if count != 0 {
                    return Err(FolioError::Parse {
                        offset: 0,
                        message: "'z' in middle of ASCII85 group".into(),
                    });
                }
                output.extend_from_slice(&[0, 0, 0, 0]);
            }
            b'!'..=b'u' => {
                group = group * 85 + (byte - b'!') as u32;
                count += 1;
                if count == 5 {
                    output.push((group >> 24) as u8);
                    output.push((group >> 16) as u8);
                    output.push((group >> 8) as u8);
                    output.push(group as u8);
                    group = 0;
                    count = 0;
                }
            }
            _ => {
                return Err(FolioError::Parse {
                    offset: 0,
                    message: format!("Invalid ASCII85 byte: 0x{:02x}", byte),
                });
            }
        }
    }

    // Handle partial final group
    if count > 1 {
        // Pad with 'u' (84) to complete the group
        for _ in count..5 {
            group = group * 85 + 84;
        }
        let bytes = group.to_be_bytes();
        for &b in &bytes[..count - 1] {
            output.push(b);
        }
    }

    Ok(output)
}

/// Encode data using ASCII85.
pub fn ascii85_encode(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(data.len() * 5 / 4 + 4);
    output.extend_from_slice(b"<~");

    let mut i = 0;
    while i + 4 <= data.len() {
        let group = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        if group == 0 {
            output.push(b'z');
        } else {
            let mut digits = [0u8; 5];
            let mut val = group;
            for d in digits.iter_mut().rev() {
                *d = (val % 85) as u8 + b'!';
                val /= 85;
            }
            output.extend_from_slice(&digits);
        }
        i += 4;
    }

    // Handle remaining bytes
    let remaining = data.len() - i;
    if remaining > 0 {
        let mut last = [0u8; 4];
        last[..remaining].copy_from_slice(&data[i..]);
        let group = u32::from_be_bytes(last);
        let mut digits = [0u8; 5];
        let mut val = group;
        for d in digits.iter_mut().rev() {
            *d = (val % 85) as u8 + b'!';
            val /= 85;
        }
        output.extend_from_slice(&digits[..remaining + 1]);
    }

    output.extend_from_slice(b"~>");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = b"Hello, World!";
        let encoded = ascii85_encode(original).unwrap();
        let decoded = ascii85_decode(&encoded).unwrap();
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_decode_known() {
        // "Man " encodes to "9jqo^"
        let decoded = ascii85_decode(b"9jqo^~>").unwrap();
        assert_eq!(&decoded, b"Man ");
    }

    #[test]
    fn test_zero_group() {
        let input = vec![0u8; 4];
        let encoded = ascii85_encode(&input).unwrap();
        assert!(encoded.windows(1).any(|w| w == b"z"));
        let decoded = ascii85_decode(&encoded).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_empty() {
        let encoded = ascii85_encode(b"").unwrap();
        let decoded = ascii85_decode(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_partial_group() {
        // Test with data not divisible by 4
        let original = b"Hello";
        let encoded = ascii85_encode(original).unwrap();
        let decoded = ascii85_decode(&encoded).unwrap();
        assert_eq!(&decoded, original);
    }
}
