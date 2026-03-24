//! PNG and TIFF predictor support for FlateDecode and LZWDecode.
//!
//! Predictors are applied after decompression to reconstruct the original data.

use super::FilterParams;
use crate::core::{FolioError, Result};

/// Apply predictor to decoded data.
pub fn apply_predictor(data: &[u8], params: &FilterParams) -> Result<Vec<u8>> {
    match params.predictor {
        1 | 0 => Ok(data.to_vec()), // No predictor
        2 => tiff_predictor(data, params),
        10..=15 => png_predictor(data, params),
        _ => Err(FolioError::UnsupportedFeature(format!(
            "Predictor type {}",
            params.predictor
        ))),
    }
}

/// TIFF predictor 2 — horizontal differencing.
fn tiff_predictor(data: &[u8], params: &FilterParams) -> Result<Vec<u8>> {
    let colors = params.colors.max(1) as usize;
    let bpc = params.bits_per_component.max(8) as usize;
    let columns = params.columns.max(1) as usize;
    let bytes_per_pixel = (colors * bpc + 7) / 8;
    let row_bytes = (columns * colors * bpc + 7) / 8;

    if data.len() % row_bytes != 0 {
        return Err(FolioError::Parse {
            offset: 0,
            message: format!(
                "TIFF predictor: data length {} not divisible by row bytes {}",
                data.len(),
                row_bytes
            ),
        });
    }

    let mut output = data.to_vec();

    for row_start in (0..output.len()).step_by(row_bytes) {
        for i in (bytes_per_pixel..row_bytes).rev() {
            // Process right-to-left is wrong; we need left-to-right
            // but we're actually undoing the predictor here
            let _ = i; // placeholder
        }
        // For 8-bit components, add previous pixel value
        if bpc == 8 {
            for i in bytes_per_pixel..row_bytes {
                output[row_start + i] =
                    output[row_start + i].wrapping_add(output[row_start + i - bytes_per_pixel]);
            }
        }
    }

    Ok(output)
}

/// PNG predictor (types 10-15) — per-row PNG filtering.
fn png_predictor(data: &[u8], params: &FilterParams) -> Result<Vec<u8>> {
    let colors = params.colors.max(1) as usize;
    let bpc = params.bits_per_component.max(8) as usize;
    let columns = params.columns.max(1) as usize;
    let bytes_per_pixel = (colors * bpc + 7) / 8;
    let row_bytes = (columns * colors * bpc + 7) / 8;
    // PNG predictor rows have a 1-byte filter type prefix
    let src_row_bytes = row_bytes + 1;

    if data.len() % src_row_bytes != 0 {
        return Err(FolioError::Parse {
            offset: 0,
            message: format!(
                "PNG predictor: data length {} not divisible by row bytes {}",
                data.len(),
                src_row_bytes
            ),
        });
    }

    let num_rows = data.len() / src_row_bytes;
    let mut output = vec![0u8; num_rows * row_bytes];
    let mut prev_row = vec![0u8; row_bytes];

    for row in 0..num_rows {
        let src_start = row * src_row_bytes;
        let filter_type = data[src_start];
        let src = &data[src_start + 1..src_start + src_row_bytes];
        let dst_start = row * row_bytes;

        match filter_type {
            0 => {
                // None
                output[dst_start..dst_start + row_bytes].copy_from_slice(src);
            }
            1 => {
                // Sub: add byte from left
                for i in 0..row_bytes {
                    let left = if i >= bytes_per_pixel {
                        output[dst_start + i - bytes_per_pixel]
                    } else {
                        0
                    };
                    output[dst_start + i] = src[i].wrapping_add(left);
                }
            }
            2 => {
                // Up: add byte from above
                for i in 0..row_bytes {
                    output[dst_start + i] = src[i].wrapping_add(prev_row[i]);
                }
            }
            3 => {
                // Average: add floor((left + above) / 2)
                for i in 0..row_bytes {
                    let left = if i >= bytes_per_pixel {
                        output[dst_start + i - bytes_per_pixel] as u16
                    } else {
                        0
                    };
                    let above = prev_row[i] as u16;
                    output[dst_start + i] = src[i].wrapping_add(((left + above) / 2) as u8);
                }
            }
            4 => {
                // Paeth
                for i in 0..row_bytes {
                    let left = if i >= bytes_per_pixel {
                        output[dst_start + i - bytes_per_pixel]
                    } else {
                        0
                    };
                    let above = prev_row[i];
                    let upper_left = if i >= bytes_per_pixel {
                        prev_row[i - bytes_per_pixel]
                    } else {
                        0
                    };
                    output[dst_start + i] = src[i].wrapping_add(paeth(left, above, upper_left));
                }
            }
            _ => {
                return Err(FolioError::Parse {
                    offset: src_start as u64,
                    message: format!("Unknown PNG filter type: {}", filter_type),
                });
            }
        }

        prev_row.copy_from_slice(&output[dst_start..dst_start + row_bytes]);
    }

    Ok(output)
}

/// Paeth predictor function (PNG spec).
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_predictor() {
        let params = FilterParams {
            predictor: 1,
            ..Default::default()
        };
        let data = vec![1, 2, 3, 4];
        let result = apply_predictor(&data, &params).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_png_none() {
        let params = FilterParams {
            predictor: 10,
            colors: 1,
            bits_per_component: 8,
            columns: 3,
            ..Default::default()
        };
        // Filter byte 0 (None) + 3 bytes per row, 1 row
        let data = vec![0, 10, 20, 30];
        let result = apply_predictor(&data, &params).unwrap();
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_png_sub() {
        let params = FilterParams {
            predictor: 11,
            colors: 1,
            bits_per_component: 8,
            columns: 4,
            ..Default::default()
        };
        // Filter byte 1 (Sub), each byte adds the one to its left
        let data = vec![1, 10, 5, 5, 5];
        let result = apply_predictor(&data, &params).unwrap();
        assert_eq!(result, vec![10, 15, 20, 25]);
    }

    #[test]
    fn test_png_up() {
        let params = FilterParams {
            predictor: 12,
            colors: 1,
            bits_per_component: 8,
            columns: 3,
            ..Default::default()
        };
        // Two rows: first with None, second with Up
        let data = vec![0, 10, 20, 30, 2, 1, 2, 3];
        let result = apply_predictor(&data, &params).unwrap();
        assert_eq!(result, vec![10, 20, 30, 11, 22, 33]);
    }

    #[test]
    fn test_paeth() {
        assert_eq!(super::paeth(7, 5, 3), 7); // p=9, pa=2, pb=4, pc=6 → a
        assert_eq!(super::paeth(0, 0, 0), 0);
    }
}
