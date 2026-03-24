//! RunLengthDecode — run-length decompression for PDF streams.

use folio_core::{FolioError, Result};

/// Decode RunLength-encoded data (PDF §7.4.5).
///
/// Format:
/// - length 0-127: copy next (length+1) bytes literally
/// - length 129-255: repeat next byte (257-length) times
/// - length 128: end of data
pub fn runlength_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < data.len() {
        let length = data[i] as i16;
        i += 1;

        if length == 128 {
            // End of data
            break;
        } else if length < 128 {
            // Copy next (length+1) bytes
            let count = (length + 1) as usize;
            if i + count > data.len() {
                return Err(FolioError::Parse {
                    offset: i as u64,
                    message: "RunLength: unexpected end of literal run".into(),
                });
            }
            output.extend_from_slice(&data[i..i + count]);
            i += count;
        } else {
            // Repeat next byte (257-length) times
            let count = (257 - length) as usize;
            if i >= data.len() {
                return Err(FolioError::Parse {
                    offset: i as u64,
                    message: "RunLength: unexpected end of repeat run".into(),
                });
            }
            let byte = data[i];
            i += 1;
            output.resize(output.len() + count, byte);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_run() {
        // length=2 means copy 3 bytes, then EOD
        let data = [2, b'A', b'B', b'C', 128];
        let decoded = runlength_decode(&data).unwrap();
        assert_eq!(&decoded, b"ABC");
    }

    #[test]
    fn test_repeat_run() {
        // length=253 means repeat 4 times (257-253=4)
        let data = [253, b'X', 128];
        let decoded = runlength_decode(&data).unwrap();
        assert_eq!(&decoded, b"XXXX");
    }

    #[test]
    fn test_mixed() {
        // Literal 2 bytes, then repeat 3 times
        let data = [1, b'A', b'B', 254, b'C', 128];
        let decoded = runlength_decode(&data).unwrap();
        assert_eq!(&decoded, b"ABCCC");
    }

    #[test]
    fn test_empty() {
        let data = [128]; // Just EOD
        let decoded = runlength_decode(&data).unwrap();
        assert!(decoded.is_empty());
    }
}
