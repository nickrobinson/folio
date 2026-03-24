//! PDF cross-reference table parsing.
//!
//! Handles both traditional xref tables and cross-reference streams (PDF 1.5+).

use super::object::PdfObject;
use super::parser;
use super::tokenizer::{Token, Tokenizer};
use crate::core::{FolioError, Result};
use indexmap::IndexMap;

/// A cross-reference entry for one object.
#[derive(Debug, Clone, Copy)]
pub enum XrefEntry {
    /// Object is in use at the given byte offset.
    InUse { offset: u64, gen_num: u16 },
    /// Object has been freed.
    Free { next_free: u32, gen_num: u16 },
    /// Object is stored in an object stream (PDF 1.5+).
    Compressed { stream_obj: u32, index: u32 },
}

/// Parsed cross-reference table with all entries and the trailer dictionary.
#[derive(Debug, Clone)]
pub struct XrefTable {
    /// Map from object number to xref entry.
    pub entries: IndexMap<u32, XrefEntry>,
    /// The trailer dictionary.
    pub trailer: IndexMap<Vec<u8>, PdfObject>,
}

/// Find the `startxref` offset from the end of a PDF file.
pub fn find_startxref(data: &[u8]) -> Result<u64> {
    let search_start = data.len().saturating_sub(1024);
    let search_region = &data[search_start..];

    let needle = b"startxref";
    let pos = search_region
        .windows(needle.len())
        .rposition(|w| w == needle)
        .ok_or_else(|| FolioError::Parse {
            offset: data.len() as u64,
            message: "Could not find startxref".into(),
        })?;

    let after = search_start + pos + needle.len();
    let mut tokenizer = Tokenizer::new_at(data, after);
    tokenizer.skip_whitespace_and_comments();

    match tokenizer.next_token()? {
        Some(Token::Integer(offset)) => Ok(offset as u64),
        other => Err(FolioError::Parse {
            offset: after as u64,
            message: format!("Expected xref offset after startxref, got {:?}", other),
        }),
    }
}

/// Parse a traditional cross-reference table starting at the given offset.
pub fn parse_xref_table(data: &[u8], offset: u64) -> Result<XrefTable> {
    let mut tokenizer = Tokenizer::new_at(data, offset as usize);

    match tokenizer.next_token()? {
        Some(Token::Keyword(ref kw)) if kw == b"xref" => {}
        _ => {
            return Err(FolioError::Parse {
                offset,
                message: "Expected 'xref' keyword".into(),
            });
        }
    }

    let mut entries = IndexMap::new();

    loop {
        tokenizer.skip_whitespace_and_comments();

        let saved = tokenizer.pos();
        match tokenizer.next_token()? {
            Some(Token::Keyword(ref kw)) if kw == b"trailer" => break,
            Some(Token::Integer(first_obj)) => {
                let count = match tokenizer.next_token()? {
                    Some(Token::Integer(n)) => n as u32,
                    _ => {
                        return Err(FolioError::Parse {
                            offset: tokenizer.pos() as u64,
                            message: "Expected object count in xref subsection".into(),
                        });
                    }
                };

                for i in 0..count {
                    tokenizer.skip_whitespace();
                    let obj_num = first_obj as u32 + i;

                    let entry_offset = match tokenizer.next_token()? {
                        Some(Token::Integer(n)) => n as u64,
                        _ => continue,
                    };
                    let gen_num = match tokenizer.next_token()? {
                        Some(Token::Integer(n)) => n as u16,
                        _ => continue,
                    };
                    let in_use = match tokenizer.next_token()? {
                        Some(Token::Keyword(ref kw)) => kw == b"n",
                        _ => continue,
                    };

                    let entry = if in_use {
                        XrefEntry::InUse {
                            offset: entry_offset,
                            gen_num,
                        }
                    } else {
                        XrefEntry::Free {
                            next_free: entry_offset as u32,
                            gen_num,
                        }
                    };

                    entries.insert(obj_num, entry);
                }
            }
            _ => {
                tokenizer.set_pos(saved);
                break;
            }
        }
    }

    let trailer = match parser::parse_object(&mut tokenizer)? {
        Some(PdfObject::Dict(d)) => d,
        _ => IndexMap::new(),
    };

    Ok(XrefTable { entries, trailer })
}

/// Parse a cross-reference stream and extract entries.
///
/// The stream dict serves as both the trailer and the xref data container.
/// See PDF spec ISO 32000-2:2020 §7.5.8.
fn parse_xref_stream(
    stream_dict: &IndexMap<Vec<u8>, PdfObject>,
    stream_data: &[u8],
) -> Result<IndexMap<u32, XrefEntry>> {
    // Get /W array: field widths [type_width, field2_width, field3_width]
    let w_array = stream_dict
        .get(b"W".as_slice())
        .and_then(|o| o.as_array())
        .ok_or_else(|| FolioError::Parse {
            offset: 0,
            message: "Xref stream missing /W array".into(),
        })?;

    if w_array.len() < 3 {
        return Err(FolioError::Parse {
            offset: 0,
            message: format!("Xref stream /W array too short: {} elements", w_array.len()),
        });
    }

    let w0 = w_array[0].as_i64().unwrap_or(0) as usize;
    let w1 = w_array[1].as_i64().unwrap_or(0) as usize;
    let w2 = w_array[2].as_i64().unwrap_or(0) as usize;
    let entry_size = w0 + w1 + w2;

    if entry_size == 0 {
        return Ok(IndexMap::new());
    }

    // Decode stream data (apply filters)
    let decoded_data = {
        let filter_names: Vec<Vec<u8>> = match stream_dict.get(b"Filter".as_slice()) {
            Some(PdfObject::Name(name)) => vec![name.clone()],
            Some(PdfObject::Array(arr)) => arr
                .iter()
                .filter_map(|o| o.as_name().map(|n| n.to_vec()))
                .collect(),
            _ => vec![],
        };

        if filter_names.is_empty() {
            stream_data.to_vec()
        } else {
            let params_list = get_decode_params(stream_dict, filter_names.len());
            crate::filters::decode_filter_chain(&filter_names, stream_data, &params_list)?
        }
    };

    // Get /Index array: [first_obj count first_obj count ...]
    // Default is [0 Size]
    let size = stream_dict
        .get(b"Size".as_slice())
        .and_then(|o| o.as_i64())
        .unwrap_or(0) as u32;

    let index_ranges: Vec<(u32, u32)> = match stream_dict.get(b"Index".as_slice()) {
        Some(PdfObject::Array(arr)) => {
            let mut ranges = Vec::new();
            let mut i = 0;
            while i + 1 < arr.len() {
                let first = arr[i].as_i64().unwrap_or(0) as u32;
                let count = arr[i + 1].as_i64().unwrap_or(0) as u32;
                ranges.push((first, count));
                i += 2;
            }
            ranges
        }
        _ => vec![(0, size)],
    };

    // Parse entries
    let mut entries = IndexMap::new();
    let mut data_pos = 0;

    for (first_obj, count) in &index_ranges {
        for i in 0..*count {
            if data_pos + entry_size > decoded_data.len() {
                break;
            }

            let obj_num = first_obj + i;

            let type_field = read_field(&decoded_data, data_pos, w0, 1); // default type=1
            let field2 = read_field(&decoded_data, data_pos + w0, w1, 0);
            let field3 = read_field(&decoded_data, data_pos + w0 + w1, w2, 0);

            data_pos += entry_size;

            let entry = match type_field {
                0 => XrefEntry::Free {
                    next_free: field2 as u32,
                    gen_num: field3 as u16,
                },
                1 => XrefEntry::InUse {
                    offset: field2,
                    gen_num: field3 as u16,
                },
                2 => XrefEntry::Compressed {
                    stream_obj: field2 as u32,
                    index: field3 as u32,
                },
                _ => continue, // Unknown type, skip
            };

            entries.insert(obj_num, entry);
        }
    }

    Ok(entries)
}

/// Read a big-endian integer field of `width` bytes from `data` at `offset`.
/// If width is 0, returns `default_value`.
fn read_field(data: &[u8], offset: usize, width: usize, default_value: u64) -> u64 {
    if width == 0 {
        return default_value;
    }
    let mut value: u64 = 0;
    for i in 0..width {
        if offset + i < data.len() {
            value = (value << 8) | data[offset + i] as u64;
        }
    }
    value
}

/// Extract DecodeParms for stream filter chain.
fn get_decode_params(
    dict: &IndexMap<Vec<u8>, PdfObject>,
    filter_count: usize,
) -> Vec<Option<crate::filters::FilterParams>> {
    match dict.get(b"DecodeParms".as_slice()) {
        Some(PdfObject::Dict(d)) => {
            vec![Some(dict_to_filter_params(d)); filter_count.max(1)]
        }
        Some(PdfObject::Array(arr)) => arr
            .iter()
            .map(|obj| obj.as_dict().map(dict_to_filter_params))
            .collect(),
        _ => vec![None; filter_count],
    }
}

fn dict_to_filter_params(dict: &IndexMap<Vec<u8>, PdfObject>) -> crate::filters::FilterParams {
    crate::filters::FilterParams {
        predictor: dict
            .get(b"Predictor".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        colors: dict
            .get(b"Colors".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        bits_per_component: dict
            .get(b"BitsPerComponent".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(8) as i32,
        columns: dict
            .get(b"Columns".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
        early_change: dict
            .get(b"EarlyChange".as_slice())
            .and_then(|o| o.as_i64())
            .unwrap_or(1) as i32,
    }
}

/// Parse all cross-reference tables (following /Prev links for incremental updates).
pub fn parse_all_xrefs(data: &[u8]) -> Result<XrefTable> {
    let startxref = find_startxref(data)?;
    let mut combined_entries = IndexMap::new();
    let mut final_trailer = IndexMap::new();
    let mut offset = startxref;
    let mut visited = std::collections::HashSet::new();

    loop {
        if visited.contains(&offset) {
            break;
        }
        visited.insert(offset);

        if offset as usize >= data.len() {
            return Err(FolioError::Parse {
                offset,
                message: "Xref offset beyond end of file".into(),
            });
        }

        let is_xref_table = data[offset as usize..].starts_with(b"xref");

        if is_xref_table {
            let table = parse_xref_table(data, offset)?;

            for (num, entry) in table.entries {
                combined_entries.entry(num).or_insert(entry);
            }

            if final_trailer.is_empty() {
                final_trailer = table.trailer.clone();
            }

            match table.trailer.get(b"Prev".as_slice()) {
                Some(PdfObject::Integer(prev)) => offset = *prev as u64,
                _ => break,
            }
        } else {
            // Cross-reference stream
            match parser::parse_indirect_object_at(data, offset as usize) {
                Ok((_id, PdfObject::Stream(stream))) => {
                    if final_trailer.is_empty() {
                        final_trailer = stream.dict.clone();
                    }

                    // Decode xref stream entries
                    match parse_xref_stream(&stream.dict, &stream.data) {
                        Ok(entries) => {
                            for (num, entry) in entries {
                                combined_entries.entry(num).or_insert(entry);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to decode xref stream: {}", e);
                        }
                    }

                    match stream.dict.get(b"Prev".as_slice()) {
                        Some(PdfObject::Integer(prev)) => offset = *prev as u64,
                        _ => break,
                    }
                }
                _ => break,
            }
        }
    }

    Ok(XrefTable {
        entries: combined_entries,
        trailer: final_trailer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_startxref() {
        let data = b"%PDF-1.4\n... content ...\nstartxref\n12345\n%%EOF";
        let offset = find_startxref(data).unwrap();
        assert_eq!(offset, 12345);
    }

    #[test]
    fn test_parse_xref_table() {
        let data = b"xref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000074 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n0\n%%EOF";
        let table = parse_xref_table(data, 0).unwrap();
        assert_eq!(table.entries.len(), 3);

        match table.entries.get(&1) {
            Some(XrefEntry::InUse { offset, gen_num }) => {
                assert_eq!(*offset, 9);
                assert_eq!(*gen_num, 0);
            }
            other => panic!("Expected InUse, got {:?}", other),
        }
    }

    #[test]
    fn test_read_field() {
        assert_eq!(read_field(&[0x01, 0x02], 0, 2, 0), 0x0102);
        assert_eq!(read_field(&[0xFF], 0, 1, 0), 255);
        assert_eq!(read_field(&[], 0, 0, 42), 42); // zero width returns default
        assert_eq!(read_field(&[0x00, 0x01, 0x00], 0, 3, 0), 256);
    }

    #[test]
    fn test_parse_xref_stream_entries() {
        // Simulate a simple xref stream: 3 entries, W=[1,2,1]
        // Entry 0: type=0 (free), next=0, gen=255
        // Entry 1: type=1 (in-use), offset=100, gen=0
        // Entry 2: type=2 (compressed), stream_obj=5, index=0
        let stream_data: Vec<u8> = vec![
            0, 0, 0, 255, // obj 0: free, next=0, gen=255
            1, 0, 100, 0, // obj 1: in-use, offset=100, gen=0
            2, 0, 5, 0, // obj 2: compressed, stream=5, index=0
        ];

        let mut dict = IndexMap::new();
        dict.insert(
            b"W".to_vec(),
            PdfObject::Array(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(2),
                PdfObject::Integer(1),
            ]),
        );
        dict.insert(b"Size".to_vec(), PdfObject::Integer(3));

        let entries = parse_xref_stream(&dict, &stream_data).unwrap();
        assert_eq!(entries.len(), 3);

        match entries.get(&0) {
            Some(XrefEntry::Free { next_free, gen_num }) => {
                assert_eq!(*next_free, 0);
                assert_eq!(*gen_num, 255);
            }
            other => panic!("Expected Free, got {:?}", other),
        }

        match entries.get(&1) {
            Some(XrefEntry::InUse { offset, gen_num }) => {
                assert_eq!(*offset, 100);
                assert_eq!(*gen_num, 0);
            }
            other => panic!("Expected InUse, got {:?}", other),
        }

        match entries.get(&2) {
            Some(XrefEntry::Compressed { stream_obj, index }) => {
                assert_eq!(*stream_obj, 5);
                assert_eq!(*index, 0);
            }
            other => panic!("Expected Compressed, got {:?}", other),
        }
    }
}
