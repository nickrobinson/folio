//! FlateDecode / FlateEncode — zlib/deflate compression.

use flate2::Compression;
use flate2::read::{DeflateDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use crate::core::{FolioError, Result};
use std::io::{Read, Write};

/// Decode FlateDecode (zlib) compressed data.
pub fn flate_decode(data: &[u8]) -> Result<Vec<u8>> {
    // Try zlib first (most PDFs use zlib-wrapped deflate)
    let mut output = Vec::new();
    let result = {
        let mut decoder = ZlibDecoder::new(data);
        decoder.read_to_end(&mut output)
    };

    match result {
        Ok(_) => Ok(output),
        Err(_) => {
            // Fall back to raw deflate (some PDFs omit the zlib header)
            output.clear();
            let mut decoder = DeflateDecoder::new(data);
            decoder
                .read_to_end(&mut output)
                .map_err(|e| FolioError::Parse {
                    offset: 0,
                    message: format!("FlateDecode failed: {}", e),
                })?;
            Ok(output)
        }
    }
}

/// Encode data using FlateDecode (zlib) compression.
pub fn flate_encode(data: &[u8]) -> Result<Vec<u8>> {
    flate_encode_level(data, Compression::default())
}

/// Encode data using FlateDecode with a specific compression level.
pub fn flate_encode_level(data: &[u8], level: Compression) -> Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), level);
    encoder.write_all(data).map_err(|e| FolioError::Parse {
        offset: 0,
        message: format!("FlateEncode failed: {}", e),
    })?;
    encoder.finish().map_err(|e| FolioError::Parse {
        offset: 0,
        message: format!("FlateEncode finish failed: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let original = b"Hello, PDF world! This is a test of flate compression.";
        let encoded = flate_encode(original).unwrap();
        let decoded = flate_decode(&encoded).unwrap();
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_roundtrip_large() {
        let original: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let encoded = flate_encode(&original).unwrap();
        assert!(encoded.len() < original.len()); // Should compress
        let decoded = flate_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_empty() {
        let encoded = flate_encode(b"").unwrap();
        let decoded = flate_decode(&encoded).unwrap();
        assert!(decoded.is_empty());
    }
}
