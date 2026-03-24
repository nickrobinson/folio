//! LZWDecode — Lempel-Ziv-Welch decompression for PDF streams.
//!
//! PDF uses a specific LZW variant as described in the PDF spec (§7.4.4).

use folio_core::{FolioError, Result};

const CLEAR_TABLE: u16 = 256;
const EOD: u16 = 257;

/// Decode LZW-compressed data.
///
/// `early_change` controls when the code width increases:
/// - 1 (default): code width increases one code early (PDF default)
/// - 0: code width increases after the code that fills the current width
pub fn lzw_decode(data: &[u8], early_change: i32) -> Result<Vec<u8>> {
    let early = early_change != 0;
    let mut reader = BitReader::new(data);
    let mut output = Vec::new();

    // Initialize table with single-byte entries
    let mut table: Vec<Vec<u8>> = (0..258)
        .map(|i| if i < 256 { vec![i as u8] } else { vec![] })
        .collect();

    let mut code_size: u8 = 9;
    let mut prev_entry: Vec<u8> = Vec::new();

    loop {
        let code = reader.read_bits(code_size)?;

        if code == CLEAR_TABLE {
            // Reset table
            table.truncate(258);
            code_size = 9;
            prev_entry.clear();

            // Read next code after clear
            let next = reader.read_bits(code_size)?;
            if next == EOD {
                break;
            }
            if (next as usize) >= table.len() {
                return Err(FolioError::Parse {
                    offset: 0,
                    message: format!("LZW: invalid code {} after clear", next),
                });
            }
            prev_entry = table[next as usize].clone();
            output.extend_from_slice(&prev_entry);
            continue;
        }

        if code == EOD {
            break;
        }

        let entry = if (code as usize) < table.len() {
            table[code as usize].clone()
        } else if code as usize == table.len() {
            // Special case: code == table size means prev_entry + first byte of prev_entry
            let mut e = prev_entry.clone();
            if let Some(&first) = prev_entry.first() {
                e.push(first);
            }
            e
        } else {
            return Err(FolioError::Parse {
                offset: 0,
                message: format!(
                    "LZW: code {} out of range (table size {})",
                    code,
                    table.len()
                ),
            });
        };

        output.extend_from_slice(&entry);

        // Add new entry to table
        if !prev_entry.is_empty() {
            let mut new_entry = prev_entry.clone();
            if let Some(&first) = entry.first() {
                new_entry.push(first);
            }
            table.push(new_entry);
        }

        prev_entry = entry;

        // Increase code size when table reaches threshold
        let threshold = if early {
            (1u32 << code_size) - 1
        } else {
            1u32 << code_size
        };
        if table.len() as u32 >= threshold && code_size < 12 {
            code_size += 1;
        }
    }

    Ok(output)
}

/// MSB-first bit reader for LZW.
struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Result<u16> {
        let mut result: u16 = 0;
        let mut remaining = count;

        while remaining > 0 {
            if self.byte_pos >= self.data.len() {
                return Err(FolioError::Parse {
                    offset: self.byte_pos as u64,
                    message: "LZW: unexpected end of data".into(),
                });
            }

            let available = 8 - self.bit_pos;
            let take = remaining.min(available);
            let shift = available - take;
            let mask = ((1u16 << take) - 1) as u8;
            let bits = (self.data[self.byte_pos] >> shift) & mask;

            result = (result << take) | bits as u16;
            remaining -= take;
            self.bit_pos += take;

            if self.bit_pos >= 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_reader() {
        // 0xFF = 11111111, 0x00 = 00000000
        let data = [0xFF, 0x00];
        let mut reader = BitReader::new(&data);
        assert_eq!(reader.read_bits(4).unwrap(), 0xF);
        assert_eq!(reader.read_bits(4).unwrap(), 0xF);
        assert_eq!(reader.read_bits(4).unwrap(), 0x0);
        assert_eq!(reader.read_bits(4).unwrap(), 0x0);
    }

    #[test]
    fn test_bit_reader_9bit() {
        // 9-bit codes spanning bytes
        let data = [0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];
        let mut reader = BitReader::new(&data);
        let code = reader.read_bits(9).unwrap();
        assert_eq!(code, CLEAR_TABLE); // 256 = 0x100
    }
}
