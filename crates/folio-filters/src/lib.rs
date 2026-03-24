//! PDF stream filter pipeline.
//!
//! PDF streams can be encoded with one or more filters (FlateDecode, ASCII85Decode, etc.).
//! This module provides encode/decode implementations for all standard PDF filters.

mod ascii85;
mod asciihex;
mod flate;
mod lzw;
mod predictor;
mod runlength;

pub use ascii85::{ascii85_decode, ascii85_encode};
pub use asciihex::{asciihex_decode, asciihex_encode};
pub use flate::{flate_decode, flate_encode};
pub use lzw::lzw_decode;
pub use predictor::apply_predictor;
pub use runlength::runlength_decode;

use folio_core::{FolioError, Result};

/// Decode data through a named PDF filter.
pub fn decode_filter(name: &[u8], data: &[u8], params: Option<&FilterParams>) -> Result<Vec<u8>> {
    match name {
        b"FlateDecode" | b"Fl" => {
            let decoded = flate_decode(data)?;
            if let Some(p) = params {
                apply_predictor(&decoded, p)
            } else {
                Ok(decoded)
            }
        }
        b"ASCII85Decode" | b"A85" => ascii85_decode(data),
        b"ASCIIHexDecode" | b"AHx" => asciihex_decode(data),
        b"LZWDecode" | b"LZW" => {
            let decoded = lzw_decode(data, params.and_then(|p| Some(p.early_change)).unwrap_or(1))?;
            if let Some(p) = params {
                apply_predictor(&decoded, p)
            } else {
                Ok(decoded)
            }
        }
        b"RunLengthDecode" | b"RL" => runlength_decode(data),
        // DCTDecode (JPEG), JPXDecode (JPEG2000), CCITTFaxDecode, JBIG2Decode
        // are image-specific and will be handled in folio-image
        _ => Err(FolioError::UnsupportedFeature(format!(
            "Filter: {}",
            String::from_utf8_lossy(name)
        ))),
    }
}

/// Decode data through a chain of filters (applied in order).
pub fn decode_filter_chain(
    filter_names: &[Vec<u8>],
    data: &[u8],
    params_list: &[Option<FilterParams>],
) -> Result<Vec<u8>> {
    let mut result = data.to_vec();
    for (i, name) in filter_names.iter().enumerate() {
        let params = params_list.get(i).and_then(|p| p.as_ref());
        result = decode_filter(name, &result, params)?;
    }
    Ok(result)
}

/// Parameters that can be associated with a filter (from DecodeParms dict).
#[derive(Debug, Clone, Default)]
pub struct FilterParams {
    /// Predictor type (1=none, 2=TIFF, 10-15=PNG)
    pub predictor: i32,
    /// Number of color components per sample
    pub colors: i32,
    /// Number of bits per color component
    pub bits_per_component: i32,
    /// Number of samples per row
    pub columns: i32,
    /// LZW early change parameter (0 or 1)
    pub early_change: i32,
}
