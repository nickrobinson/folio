//! PDF font handling — loading, metrics, encoding, and Unicode mapping.

mod cmap;
mod encoding;
mod font;
mod metrics;

pub use cmap::ToUnicodeCMap;
pub use encoding::{Encoding, PdfEncoding, decode_text};
pub use font::{FontType, PdfFont};
pub use metrics::STANDARD_14_METRICS;
