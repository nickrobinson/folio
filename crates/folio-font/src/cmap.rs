//! ToUnicode CMap parsing — maps character codes to Unicode strings.
//!
//! ToUnicode CMaps are the primary mechanism for extracting Unicode text
//! from PDFs. They are embedded as streams in the font dictionary.

use folio_core::Result;
use std::collections::HashMap;

/// A parsed ToUnicode CMap.
#[derive(Debug, Clone, Default)]
pub struct ToUnicodeCMap {
    /// Direct character code -> Unicode string mappings.
    mappings: HashMap<u32, String>,
    /// Range mappings: (start_code, end_code, start_unicode).
    ranges: Vec<(u32, u32, u32)>,
}

impl ToUnicodeCMap {
    /// Parse a ToUnicode CMap from its text content.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let text = String::from_utf8_lossy(data);
        let mut cmap = ToUnicodeCMap::default();

        let mut lines = text.lines().peekable();

        while let Some(line) = lines.next() {
            let line = line.trim();

            if line.ends_with("beginbfchar") {
                // Parse individual character mappings
                while let Some(mapping_line) = lines.next() {
                    let mapping_line = mapping_line.trim();
                    if mapping_line.contains("endbfchar") {
                        break;
                    }
                    if let Some((code, unicode)) = parse_bfchar_line(mapping_line) {
                        cmap.mappings.insert(code, unicode);
                    }
                }
            } else if line.ends_with("beginbfrange") {
                // Parse range mappings
                while let Some(range_line) = lines.next() {
                    let range_line = range_line.trim();
                    if range_line.contains("endbfrange") {
                        break;
                    }
                    if let Some((start, end, unicode_start)) = parse_bfrange_line(range_line) {
                        cmap.ranges.push((start, end, unicode_start));
                    }
                }
            }
        }

        Ok(cmap)
    }

    /// Look up a character code in this CMap.
    pub fn lookup(&self, code: u32) -> Option<String> {
        // Check direct mappings first
        if let Some(s) = self.mappings.get(&code) {
            return Some(s.clone());
        }

        // Check range mappings
        for &(start, end, unicode_start) in &self.ranges {
            if code >= start && code <= end {
                let offset = code - start;
                if let Some(ch) = char::from_u32(unicode_start + offset) {
                    return Some(ch.to_string());
                }
            }
        }

        None
    }

    /// Decode a byte sequence using this CMap.
    ///
    /// Tries 2-byte codes first, falls back to 1-byte.
    pub fn decode(&self, data: &[u8]) -> String {
        let mut result = String::new();
        let mut i = 0;

        while i < data.len() {
            // Try 2-byte code first (common in CID fonts)
            if i + 1 < data.len() {
                let code2 = ((data[i] as u32) << 8) | (data[i + 1] as u32);
                if let Some(s) = self.lookup(code2) {
                    result.push_str(&s);
                    i += 2;
                    continue;
                }
            }

            // Try 1-byte code
            let code1 = data[i] as u32;
            if let Some(s) = self.lookup(code1) {
                result.push_str(&s);
            } else {
                // Fallback: try as ASCII
                if data[i] >= 0x20 && data[i] <= 0x7E {
                    result.push(data[i] as char);
                }
            }
            i += 1;
        }

        result
    }

    /// Whether this CMap has any mappings.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty() && self.ranges.is_empty()
    }
}

/// Parse a single bfchar mapping line: <XXXX> <YYYY>
fn parse_bfchar_line(line: &str) -> Option<(u32, String)> {
    let parts: Vec<&str> = line.split('<').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let code_hex = parts[0].split('>').next()?;
    let unicode_hex = parts[1].split('>').next()?;

    let code = u32::from_str_radix(code_hex.trim(), 16).ok()?;
    let unicode_str = hex_to_unicode_string(unicode_hex.trim())?;

    Some((code, unicode_str))
}

/// Parse a bfrange line: <XXXX> <YYYY> <ZZZZ>
fn parse_bfrange_line(line: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = line.split('<').filter(|s| !s.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    let start_hex = parts[0].split('>').next()?;
    let end_hex = parts[1].split('>').next()?;
    let unicode_hex = parts[2].split('>').next()?;

    let start = u32::from_str_radix(start_hex.trim(), 16).ok()?;
    let end = u32::from_str_radix(end_hex.trim(), 16).ok()?;
    let unicode_start = u32::from_str_radix(unicode_hex.trim(), 16).ok()?;

    Some((start, end, unicode_start))
}

/// Convert a hex string to a Unicode string.
/// E.g., "0048006500" -> "He"
fn hex_to_unicode_string(hex: &str) -> Option<String> {
    let hex = hex.trim();
    if hex.len() <= 4 {
        // Single code point
        let cp = u32::from_str_radix(hex, 16).ok()?;
        char::from_u32(cp).map(|c| c.to_string())
    } else {
        // Multiple code points (each 4 hex digits)
        let mut result = String::new();
        let mut i = 0;
        while i + 3 < hex.len() {
            if let Ok(cp) = u32::from_str_radix(&hex[i..i + 4], 16) {
                if let Some(c) = char::from_u32(cp) {
                    result.push(c);
                }
            }
            i += 4;
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bfchar() {
        let cmap_data = br#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
1 begincodespacerange
<00> <FF>
endcodespacerange
3 beginbfchar
<01> <0048>
<02> <0065>
<03> <006C>
endbfchar
endcmap
"#;
        let cmap = ToUnicodeCMap::parse(cmap_data).unwrap();
        assert_eq!(cmap.lookup(1), Some("H".into()));
        assert_eq!(cmap.lookup(2), Some("e".into()));
        assert_eq!(cmap.lookup(3), Some("l".into()));
    }

    #[test]
    fn test_parse_bfrange() {
        let cmap_data = br#"
1 beginbfrange
<0041> <005A> <0041>
endbfrange
"#;
        let cmap = ToUnicodeCMap::parse(cmap_data).unwrap();
        assert_eq!(cmap.lookup(0x41), Some("A".into()));
        assert_eq!(cmap.lookup(0x42), Some("B".into()));
        assert_eq!(cmap.lookup(0x5A), Some("Z".into()));
        assert_eq!(cmap.lookup(0x5B), None);
    }

    #[test]
    fn test_decode() {
        let cmap_data = br#"
3 beginbfchar
<48> <0048>
<65> <0065>
<6C> <006C>
endbfchar
"#;
        let cmap = ToUnicodeCMap::parse(cmap_data).unwrap();
        assert_eq!(cmap.decode(b"Hel"), "Hel");
    }
}
