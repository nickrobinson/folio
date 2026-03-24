//! PDF text encoding tables and decoding.
//!
//! PDF uses several legacy encodings for mapping character codes to glyphs.
//! The most common are WinAnsiEncoding and MacRomanEncoding.

/// Known PDF text encodings.
#[derive(Debug, Clone, PartialEq)]
pub enum PdfEncoding {
    StandardEncoding,
    MacRomanEncoding,
    WinAnsiEncoding,
    PDFDocEncoding,
    MacExpertEncoding,
    Identity,
    Custom(Vec<Option<String>>),
}

/// An encoding that can map character codes to Unicode strings.
#[derive(Debug)]
pub struct Encoding {
    table: Vec<Option<String>>,
}

impl Encoding {
    /// Create an encoding from a PDF encoding name.
    pub fn from_name(name: &[u8]) -> Self {
        match name {
            b"WinAnsiEncoding" => Self::win_ansi(),
            b"MacRomanEncoding" => Self::mac_roman(),
            b"StandardEncoding" => Self::standard(),
            b"ZapfDingbatsEncoding" => Self::zapf_dingbats(),
            b"SymbolEncoding" => Self::symbol(),
            _ => Self::win_ansi(),
        }
    }

    /// Create WinAnsiEncoding (Windows code page 1252).
    pub fn win_ansi() -> Self {
        let mut table = vec![None; 256];
        // Use Windows-1252 (cp1252) for the full 0-255 range.
        // encoding_rs handles this correctly.
        let cp1252_bytes: Vec<u8> = (0..=255u16).map(|i| i as u8).collect();
        let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&cp1252_bytes);
        for (i, ch) in decoded.chars().enumerate() {
            if i < 256 && (ch != '\0' || i == 0) {
                table[i] = Some(ch.to_string());
            }
        }
        // Ensure control characters are handled
        table[0x09] = Some("\t".into());
        table[0x0A] = Some("\n".into());
        table[0x0D] = Some("\r".into());
        table[0x20] = Some(" ".into());
        Self { table }
    }

    /// Create MacRomanEncoding.
    pub fn mac_roman() -> Self {
        let mut table = vec![None; 256];
        // Use macOS Roman encoding via encoding_rs.
        let mac_bytes: Vec<u8> = (0..=255u16).map(|i| i as u8).collect();
        let (decoded, _, _) = encoding_rs::MACINTOSH.decode(&mac_bytes);
        for (i, ch) in decoded.chars().enumerate() {
            if i < 256 {
                table[i] = Some(ch.to_string());
            }
        }
        table[0x09] = Some("\t".into());
        table[0x0A] = Some("\n".into());
        table[0x0D] = Some("\r".into());
        table[0x20] = Some(" ".into());
        Self { table }
    }

    /// Create StandardEncoding (Adobe's standard encoding for Type 1 fonts).
    pub fn standard() -> Self {
        // StandardEncoding: ASCII 0x20-0x7E same as ASCII, plus specific mappings
        // for the 0x80-0xFF range. Use WinAnsi as base and override specific positions.
        let mut enc = Self::win_ansi();
        // Key differences from WinAnsi in the upper range:
        enc.table[0x60] = Some("\u{2018}".into()); // quoteleft
        enc.table[0x27] = Some("\u{2019}".into()); // quoteright
        enc.table[0xA1] = Some("\u{00A1}".into()); // exclamdown
        enc.table[0xA2] = Some("\u{00A2}".into()); // cent
        enc.table[0xA3] = Some("\u{00A3}".into()); // sterling
        enc.table[0xA4] = Some("\u{2044}".into()); // fraction
        enc.table[0xA5] = Some("\u{00A5}".into()); // yen
        enc.table[0xA6] = Some("\u{0192}".into()); // florin
        enc.table[0xA7] = Some("\u{00A7}".into()); // section
        enc.table[0xA8] = Some("\u{00A4}".into()); // currency
        enc.table[0xAC] = Some("\u{FB01}".into()); // fi
        enc.table[0xAD] = Some("\u{FB02}".into()); // fl
        enc.table[0xB0] = Some("\u{2013}".into()); // endash
        enc.table[0xB1] = Some("\u{2020}".into()); // dagger
        enc.table[0xB2] = Some("\u{2021}".into()); // daggerdbl
        enc.table[0xB3] = Some("\u{00B7}".into()); // periodcentered
        enc.table[0xB7] = Some("\u{2022}".into()); // bullet
        enc.table[0xB8] = Some("\u{201A}".into()); // quotesinglbase
        enc.table[0xB9] = Some("\u{201E}".into()); // quotedblbase
        enc.table[0xBA] = Some("\u{201D}".into()); // quotedblright
        enc.table[0xBB] = Some("\u{00BB}".into()); // guillemotright
        enc.table[0xBC] = Some("\u{2026}".into()); // ellipsis
        enc.table[0xBD] = Some("\u{2030}".into()); // perthousand
        enc.table[0xC1] = Some("\u{0060}".into()); // grave
        enc.table[0xC2] = Some("\u{00B4}".into()); // acute
        enc.table[0xC3] = Some("\u{02C6}".into()); // circumflex
        enc.table[0xC4] = Some("\u{02DC}".into()); // tilde
        enc.table[0xC5] = Some("\u{00AF}".into()); // macron
        enc.table[0xC6] = Some("\u{02D8}".into()); // breve
        enc.table[0xC7] = Some("\u{02D9}".into()); // dotaccent
        enc.table[0xC8] = Some("\u{00A8}".into()); // dieresis
        enc.table[0xCA] = Some("\u{02DA}".into()); // ring
        enc.table[0xCB] = Some("\u{00B8}".into()); // cedilla
        enc.table[0xCD] = Some("\u{02DD}".into()); // hungarumlaut
        enc.table[0xCE] = Some("\u{02DB}".into()); // ogonek
        enc.table[0xCF] = Some("\u{02C7}".into()); // caron
        enc.table[0xD0] = Some("\u{2014}".into()); // emdash
        enc.table[0xE1] = Some("\u{00C6}".into()); // AE
        enc.table[0xE3] = Some("\u{00AA}".into()); // ordfeminine
        enc.table[0xE8] = Some("\u{0141}".into()); // Lslash
        enc.table[0xE9] = Some("\u{00D8}".into()); // Oslash
        enc.table[0xEA] = Some("\u{0152}".into()); // OE
        enc.table[0xEB] = Some("\u{00BA}".into()); // ordmasculine
        enc.table[0xF1] = Some("\u{00E6}".into()); // ae
        enc.table[0xF5] = Some("\u{0131}".into()); // dotlessi
        enc.table[0xF8] = Some("\u{0142}".into()); // lslash
        enc.table[0xF9] = Some("\u{00F8}".into()); // oslash
        enc.table[0xFA] = Some("\u{0153}".into()); // oe
        enc.table[0xFB] = Some("\u{00DF}".into()); // germandbls
        enc
    }

    /// Create ZapfDingbats encoding.
    ///
    /// ZapfDingbats uses its own character set where ASCII letters map to
    /// various symbols (arrows, stars, diamonds, etc.).
    pub fn zapf_dingbats() -> Self {
        let mut table = vec![None; 256];
        // Space
        table[0x20] = Some(" ".into());
        // Key ZapfDingbats mappings (0x21-0x7E range = symbol glyphs)
        let mappings: &[(u8, char)] = &[
            (0x21, '\u{2701}'), // upper blade scissors
            (0x22, '\u{2702}'), // black scissors
            (0x23, '\u{2703}'), // lower blade scissors
            (0x24, '\u{2704}'), // white scissors
            (0x25, '\u{260E}'), // black telephone
            (0x26, '\u{2706}'), // telephone location sign
            (0x27, '\u{2707}'), // tape drive
            (0x28, '\u{2708}'), // airplane
            (0x29, '\u{2709}'), // envelope
            (0x2A, '\u{261B}'), // black right pointing index
            (0x2B, '\u{261E}'), // white right pointing index
            (0x2C, '\u{270C}'), // victory hand
            (0x2D, '\u{270D}'), // writing hand
            (0x2E, '\u{270E}'), // lower right pencil
            (0x2F, '\u{270F}'), // pencil
            (0x30, '\u{2710}'), // upper right pencil
            (0x31, '\u{2711}'), // white nib
            (0x32, '\u{2712}'), // black nib
            (0x33, '\u{2713}'), // check mark
            (0x34, '\u{2714}'), // heavy check mark
            (0x35, '\u{2715}'), // multiplication x
            (0x36, '\u{2716}'), // heavy multiplication x
            (0x37, '\u{2717}'), // ballot x
            (0x38, '\u{2718}'), // heavy ballot x
            (0x39, '\u{2719}'), // outlined Greek cross
            (0x3A, '\u{271A}'), // heavy Greek cross
            (0x3B, '\u{271B}'), // open centre cross
            (0x3C, '\u{271C}'), // heavy open centre cross
            (0x3D, '\u{271D}'), // Latin cross
            (0x3E, '\u{271E}'), // shadowed white Latin cross
            (0x3F, '\u{271F}'), // outlined Latin cross
            (0x40, '\u{2720}'), // Maltese cross
            (0x41, '\u{2721}'), // star of David
            (0x42, '\u{2722}'), // four teardrop-spoked asterisk
            (0x43, '\u{2723}'), // four balloon-spoked asterisk
            (0x44, '\u{2724}'), // heavy four balloon-spoked asterisk
            (0x45, '\u{2725}'), // four club-spoked asterisk
            (0x46, '\u{2726}'), // black four pointed star
            (0x47, '\u{2727}'), // white four pointed star
            (0x48, '\u{2605}'), // black star
            (0x49, '\u{2729}'), // stress outlined white star
            (0x4A, '\u{272A}'), // circled white star
            (0x4B, '\u{272B}'), // open centre black star
            (0x4C, '\u{272C}'), // black centre white star
            (0x4D, '\u{272D}'), // outlined black star
            (0x4E, '\u{272E}'), // heavy outlined black star
            (0x4F, '\u{272F}'), // pinwheel star
            (0x50, '\u{2730}'), // shadowed white star
            (0x51, '\u{2731}'), // heavy asterisk
            (0x52, '\u{2732}'), // open centre asterisk
            (0x53, '\u{2733}'), // eight spoked asterisk
            (0x54, '\u{2734}'), // eight pointed black star
            (0x55, '\u{2735}'), // eight pointed pinwheel star
            (0x56, '\u{2736}'), // six pointed black star
            (0x57, '\u{2737}'), // eight pointed rectilinear black star
            (0x58, '\u{2738}'), // heavy eight pointed rectilinear black star
            (0x59, '\u{2739}'), // twelve pointed black star
            (0x5A, '\u{273A}'), // sixteen pointed asterisk
            (0x5B, '\u{273B}'), // teardrop-spoked asterisk
            (0x5C, '\u{273C}'), // open centre teardrop-spoked asterisk
            (0x5D, '\u{273D}'), // heavy teardrop-spoked asterisk
            (0x5E, '\u{273E}'), // six petalled black and white florette
            (0x5F, '\u{273F}'), // black florette
            (0x60, '\u{2740}'), // white florette
            (0x61, '\u{2741}'), // eight petalled outlined black florette
            (0x62, '\u{2742}'), // circled open centre eight pointed star
            (0x63, '\u{2743}'), // heavy teardrop-spoked pinwheel asterisk
            (0x64, '\u{2744}'), // snowflake
            (0x65, '\u{2745}'), // tight trifoliate snowflake
            (0x66, '\u{2746}'), // heavy chevron snowflake
            (0x67, '\u{2747}'), // sparkle
            (0x68, '\u{2748}'), // heavy sparkle
            (0x69, '\u{2749}'), // balloon-spoked asterisk
            (0x6A, '\u{274A}'), // eight teardrop-spoked propeller asterisk
            (0x6B, '\u{274B}'), // heavy eight teardrop-spoked propeller asterisk
            // Key mapping: 0x6C-0x7E
            (0x6C, '\u{25CF}'), // black circle
            (0x6D, '\u{274D}'), // shadowed white circle
            (0x6E, '\u{25A0}'), // black square
            (0x6F, '\u{274F}'), // lower right drop-shadowed white square
            (0x70, '\u{2750}'), // upper right drop-shadowed white square
            (0x71, '\u{2751}'), // lower right shadowed white square
            (0x72, '\u{2752}'), // upper right shadowed white square
            (0x73, '\u{25B2}'), // black up-pointing triangle
            (0x74, '\u{25BC}'), // black down-pointing triangle
            (0x75, '\u{25C6}'), // BLACK DIAMOND — the key mapping!
            (0x76, '\u{2756}'), // black diamond minus white x
            (0x77, '\u{25D7}'), // right half black circle
            (0x78, '\u{2758}'), // light vertical bar
            (0x79, '\u{2759}'), // medium vertical bar
            (0x7A, '\u{275A}'), // heavy vertical bar
            (0x7B, '\u{275B}'), // heavy single turned comma quotation mark ornament
            (0x7C, '\u{275C}'), // heavy single comma quotation mark ornament
            (0x7D, '\u{275D}'), // heavy double turned comma quotation mark ornament
            (0x7E, '\u{275E}'), // heavy double comma quotation mark ornament
        ];
        for &(code, ch) in mappings {
            table[code as usize] = Some(ch.to_string());
        }
        // 0x80-0x9F range (some dingbats continue here)
        let high_mappings: &[(u8, char)] = &[
            (0x80, '\u{2768}'),
            (0x81, '\u{2769}'),
            (0x82, '\u{276A}'),
            (0x83, '\u{276B}'),
            (0x84, '\u{276C}'),
            (0x85, '\u{276D}'),
            (0x86, '\u{276E}'),
            (0x87, '\u{276F}'),
            (0x88, '\u{2770}'),
            (0x89, '\u{2771}'),
            (0x8A, '\u{2772}'),
            (0x8B, '\u{2773}'),
            (0x8C, '\u{2774}'),
            (0x8D, '\u{2775}'),
            (0xA1, '\u{2761}'),
            (0xA2, '\u{2762}'),
            (0xA3, '\u{2763}'),
            (0xA4, '\u{2764}'),
            (0xA5, '\u{2765}'),
            (0xA6, '\u{2766}'),
            (0xA7, '\u{2767}'),
            (0xB1, '\u{2460}'),
            (0xB2, '\u{2461}'),
            (0xB3, '\u{2462}'),
            (0xB4, '\u{2463}'),
            (0xB5, '\u{2464}'),
            (0xB6, '\u{2465}'),
            (0xB7, '\u{2466}'),
            (0xB8, '\u{2467}'),
            (0xB9, '\u{2468}'),
            (0xBA, '\u{2469}'),
            (0xC0, '\u{2776}'),
            (0xC1, '\u{2777}'),
            (0xC2, '\u{2778}'),
            (0xC3, '\u{2779}'),
            (0xC4, '\u{277A}'),
            (0xC5, '\u{277B}'),
            (0xC6, '\u{277C}'),
            (0xC7, '\u{277D}'),
            (0xC8, '\u{277E}'),
            (0xC9, '\u{277F}'),
            (0xD1, '\u{2780}'),
            (0xD2, '\u{2781}'),
            (0xD3, '\u{2782}'),
            (0xD4, '\u{2783}'),
            (0xD5, '\u{2784}'),
            (0xD6, '\u{2785}'),
            (0xD7, '\u{2786}'),
            (0xD8, '\u{2787}'),
            (0xD9, '\u{2788}'),
            (0xDA, '\u{2789}'),
            (0xE1, '\u{278A}'),
            (0xE2, '\u{278B}'),
            (0xE3, '\u{278C}'),
            (0xE4, '\u{278D}'),
            (0xE5, '\u{278E}'),
            (0xE6, '\u{278F}'),
            (0xE7, '\u{2790}'),
            (0xE8, '\u{2791}'),
            (0xE9, '\u{2792}'),
            (0xEA, '\u{2793}'),
            (0xF1, '\u{2794}'),
            (0xF2, '\u{2192}'), // rightwards arrow
            (0xF3, '\u{2194}'), // left right arrow
            (0xF4, '\u{2195}'), // up down arrow
        ];
        for &(code, ch) in high_mappings {
            table[code as usize] = Some(ch.to_string());
        }
        Self { table }
    }

    /// Create Symbol encoding (Adobe Symbol font).
    pub fn symbol() -> Self {
        let mut table = vec![None; 256];
        // ASCII printable range is mostly identity for Symbol
        // but with Greek letters and math symbols instead of Latin
        table[0x20] = Some(" ".into());
        let mappings: &[(u8, char)] = &[
            (0x21, '!'),
            (0x22, '\u{2200}'), // for all
            (0x23, '#'),
            (0x24, '\u{2203}'), // there exists
            (0x25, '%'),
            (0x26, '&'),
            (0x27, '\u{220B}'), // contains
            (0x28, '('),
            (0x29, ')'),
            (0x2A, '*'),
            (0x2B, '+'),
            (0x2C, ','),
            (0x2D, '\u{2212}'), // minus sign
            (0x2E, '.'),
            (0x2F, '/'),
            (0x30, '0'),
            (0x31, '1'),
            (0x32, '2'),
            (0x33, '3'),
            (0x34, '4'),
            (0x35, '5'),
            (0x36, '6'),
            (0x37, '7'),
            (0x38, '8'),
            (0x39, '9'),
            (0x3A, ':'),
            (0x3B, ';'),
            (0x3C, '<'),
            (0x3D, '='),
            (0x3E, '>'),
            (0x3F, '?'),
            (0x40, '\u{2245}'), // approximately equal
            (0x41, '\u{0391}'), // Alpha
            (0x42, '\u{0392}'), // Beta
            (0x43, '\u{03A7}'), // Chi
            (0x44, '\u{0394}'), // Delta
            (0x45, '\u{0395}'), // Epsilon
            (0x46, '\u{03A6}'), // Phi
            (0x47, '\u{0393}'), // Gamma
            (0x48, '\u{0397}'), // Eta
            (0x49, '\u{0399}'), // Iota
            (0x4B, '\u{039A}'), // Kappa
            (0x4C, '\u{039B}'), // Lambda
            (0x4D, '\u{039C}'), // Mu
            (0x4E, '\u{039D}'), // Nu
            (0x4F, '\u{039F}'), // Omicron
            (0x50, '\u{03A0}'), // Pi
            (0x51, '\u{0398}'), // Theta
            (0x52, '\u{03A1}'), // Rho
            (0x53, '\u{03A3}'), // Sigma
            (0x54, '\u{03A4}'), // Tau
            (0x55, '\u{03A5}'), // Upsilon
            (0x57, '\u{03A9}'), // Omega
            (0x58, '\u{039E}'), // Xi
            (0x59, '\u{03A8}'), // Psi
            (0x5A, '\u{0396}'), // Zeta
            (0x5B, '['),
            (0x5D, ']'),
            (0x5E, '\u{22A5}'), // perpendicular
            (0x5F, '_'),
            (0x61, '\u{03B1}'), // alpha
            (0x62, '\u{03B2}'), // beta
            (0x63, '\u{03C7}'), // chi
            (0x64, '\u{03B4}'), // delta
            (0x65, '\u{03B5}'), // epsilon
            (0x66, '\u{03C6}'), // phi
            (0x67, '\u{03B3}'), // gamma
            (0x68, '\u{03B7}'), // eta
            (0x69, '\u{03B9}'), // iota
            (0x6B, '\u{03BA}'), // kappa
            (0x6C, '\u{03BB}'), // lambda
            (0x6D, '\u{03BC}'), // mu
            (0x6E, '\u{03BD}'), // nu
            (0x6F, '\u{03BF}'), // omicron
            (0x70, '\u{03C0}'), // pi
            (0x71, '\u{03B8}'), // theta
            (0x72, '\u{03C1}'), // rho
            (0x73, '\u{03C3}'), // sigma
            (0x74, '\u{03C4}'), // tau
            (0x75, '\u{03C5}'), // upsilon
            (0x77, '\u{03C9}'), // omega
            (0x78, '\u{03BE}'), // xi
            (0x79, '\u{03C8}'), // psi
            (0x7A, '\u{03B6}'), // zeta
            (0x7B, '{'),
            (0x7C, '|'),
            (0x7D, '}'),
            (0x7E, '\u{223C}'), // tilde operator
            (0xA0, '\u{20AC}'), // Euro
            (0xB1, '\u{00B1}'), // plus-minus
            (0xB4, '\u{00D7}'), // multiplication
            (0xB5, '\u{221D}'), // proportional to
            (0xB6, '\u{2202}'), // partial differential
            (0xB7, '\u{2022}'), // bullet
            (0xB8, '\u{00F7}'), // division
            (0xB9, '\u{2260}'), // not equal
            (0xBA, '\u{2261}'), // identical
            (0xBB, '\u{2248}'), // almost equal
            (0xBC, '\u{2026}'), // ellipsis
            (0xC0, '\u{2135}'), // alef
            (0xC1, '\u{2111}'), // imaginary part
            (0xC2, '\u{211C}'), // real part
            (0xC3, '\u{2118}'), // Weierstrass p
            (0xC5, '\u{2297}'), // circled times
            (0xC6, '\u{2295}'), // circled plus
            (0xC7, '\u{2205}'), // empty set
            (0xC8, '\u{2229}'), // intersection
            (0xC9, '\u{222A}'), // union
            (0xCA, '\u{2283}'), // superset
            (0xCB, '\u{2287}'), // superset or equal
            (0xCC, '\u{2284}'), // not subset
            (0xCD, '\u{2282}'), // subset
            (0xCE, '\u{2286}'), // subset or equal
            (0xCF, '\u{2208}'), // element of
            (0xD0, '\u{2209}'), // not element of
            (0xD1, '\u{2220}'), // angle
            (0xD2, '\u{2207}'), // nabla
            (0xD5, '\u{220F}'), // product
            (0xD6, '\u{221A}'), // square root
            (0xE0, '\u{25CA}'), // lozenge
            (0xE5, '\u{2211}'), // summation
            (0xF2, '\u{222B}'), // integral
            (0xF5, '\u{221E}'), // infinity
        ];
        for &(code, ch) in mappings {
            table[code as usize] = Some(ch.to_string());
        }
        Self { table }
    }

    /// Apply a Differences array to modify this encoding.
    pub fn apply_differences(&mut self, differences: &[folio_cos::PdfObject]) {
        let mut code = 0u16;
        for item in differences {
            match item {
                folio_cos::PdfObject::Integer(n) => code = *n as u16,
                folio_cos::PdfObject::Name(name) => {
                    if (code as usize) < self.table.len() {
                        let unicode = glyph_name_to_unicode(name);
                        self.table[code as usize] = Some(unicode);
                    }
                    code += 1;
                }
                _ => {}
            }
        }
    }

    /// Map a character code to a Unicode string.
    pub fn decode_char(&self, code: u8) -> Option<&str> {
        self.table.get(code as usize).and_then(|s| s.as_deref())
    }

    /// Decode a byte sequence to a Unicode string.
    pub fn decode_bytes(&self, data: &[u8]) -> String {
        let mut result = String::new();
        for &byte in data {
            match self.decode_char(byte) {
                Some(s) => result.push_str(s),
                None => result.push(char::REPLACEMENT_CHARACTER),
            }
        }
        result
    }
}

/// Map a glyph name to its Unicode string representation.
///
/// Covers the Adobe Glyph List (AGL) entries most commonly found in PDFs.
fn glyph_name_to_unicode(name: &[u8]) -> String {
    let name_str = std::str::from_utf8(name).unwrap_or("");
    match name_str {
        // Basic ASCII-named glyphs
        "space" | "nbspace" => " ".into(),
        "exclam" => "!".into(),
        "quotedbl" => "\"".into(),
        "numbersign" => "#".into(),
        "dollar" => "$".into(),
        "percent" => "%".into(),
        "ampersand" => "&".into(),
        "quotesingle" => "'".into(),
        "parenleft" => "(".into(),
        "parenright" => ")".into(),
        "asterisk" => "*".into(),
        "plus" => "+".into(),
        "comma" => ",".into(),
        "hyphen" | "minus" | "hyphenchar" => "-".into(),
        "period" => ".".into(),
        "slash" => "/".into(),
        "zero" => "0".into(),
        "one" => "1".into(),
        "two" => "2".into(),
        "three" => "3".into(),
        "four" => "4".into(),
        "five" => "5".into(),
        "six" => "6".into(),
        "seven" => "7".into(),
        "eight" => "8".into(),
        "nine" => "9".into(),
        "colon" => ":".into(),
        "semicolon" => ";".into(),
        "less" => "<".into(),
        "equal" => "=".into(),
        "greater" => ">".into(),
        "question" => "?".into(),
        "at" => "@".into(),
        "bracketleft" => "[".into(),
        "backslash" => "\\".into(),
        "bracketright" => "]".into(),
        "asciicircum" => "^".into(),
        "underscore" => "_".into(),
        "grave" | "quoteleft" => "\u{2018}".into(),
        "braceleft" => "{".into(),
        "bar" => "|".into(),
        "braceright" => "}".into(),
        "asciitilde" => "~".into(),

        // Typographic quotes and dashes
        "quoteright" => "\u{2019}".into(),
        "quotedblleft" => "\u{201C}".into(),
        "quotedblright" => "\u{201D}".into(),
        "quotedblbase" => "\u{201E}".into(),
        "quotesinglbase" => "\u{201A}".into(),
        "guillemotleft" | "guilsinglleft" => "\u{00AB}".into(),
        "guillemotright" | "guilsinglright" => "\u{00BB}".into(),

        "endash" => "\u{2013}".into(),
        "emdash" => "\u{2014}".into(),
        "bullet" => "\u{2022}".into(),
        "ellipsis" => "\u{2026}".into(),
        "dagger" => "\u{2020}".into(),
        "daggerdbl" => "\u{2021}".into(),
        "perthousand" => "\u{2030}".into(),
        "trademark" => "\u{2122}".into(),
        "copyright" => "\u{00A9}".into(),
        "registered" => "\u{00AE}".into(),

        // Special characters
        "fi" => "\u{FB01}".into(),
        "fl" => "\u{FB02}".into(),
        "ff" => "\u{FB00}".into(),
        "ffi" => "\u{FB03}".into(),
        "ffl" => "\u{FB04}".into(),
        "lozenge" => "\u{25CA}".into(),
        "Euro" => "\u{20AC}".into(),
        "degree" => "\u{00B0}".into(),
        "section" => "\u{00A7}".into(),
        "paragraph" | "pilcrow" => "\u{00B6}".into(),
        "fraction" => "\u{2044}".into(),
        "florin" => "\u{0192}".into(),

        // Accented Latin characters
        "Agrave" => "\u{00C0}".into(),
        "Aacute" => "\u{00C1}".into(),
        "Acircumflex" => "\u{00C2}".into(),
        "Atilde" => "\u{00C3}".into(),
        "Adieresis" => "\u{00C4}".into(),
        "Aring" => "\u{00C5}".into(),
        "AE" => "\u{00C6}".into(),
        "Ccedilla" => "\u{00C7}".into(),
        "Egrave" => "\u{00C8}".into(),
        "Eacute" => "\u{00C9}".into(),
        "Ecircumflex" => "\u{00CA}".into(),
        "Edieresis" => "\u{00CB}".into(),
        "Igrave" => "\u{00CC}".into(),
        "Iacute" => "\u{00CD}".into(),
        "Icircumflex" => "\u{00CE}".into(),
        "Idieresis" => "\u{00CF}".into(),
        "Eth" => "\u{00D0}".into(),
        "Ntilde" => "\u{00D1}".into(),
        "Ograve" => "\u{00D2}".into(),
        "Oacute" => "\u{00D3}".into(),
        "Ocircumflex" => "\u{00D4}".into(),
        "Otilde" => "\u{00D5}".into(),
        "Odieresis" => "\u{00D6}".into(),
        "Oslash" => "\u{00D8}".into(),
        "Ugrave" => "\u{00D9}".into(),
        "Uacute" => "\u{00DA}".into(),
        "Ucircumflex" => "\u{00DB}".into(),
        "Udieresis" => "\u{00DC}".into(),
        "Yacute" => "\u{00DD}".into(),
        "Thorn" => "\u{00DE}".into(),
        "germandbls" => "\u{00DF}".into(),
        "agrave" => "\u{00E0}".into(),
        "aacute" => "\u{00E1}".into(),
        "acircumflex" => "\u{00E2}".into(),
        "atilde" => "\u{00E3}".into(),
        "adieresis" => "\u{00E4}".into(),
        "aring" => "\u{00E5}".into(),
        "ae" => "\u{00E6}".into(),
        "ccedilla" => "\u{00E7}".into(),
        "egrave" => "\u{00E8}".into(),
        "eacute" => "\u{00E9}".into(),
        "ecircumflex" => "\u{00EA}".into(),
        "edieresis" => "\u{00EB}".into(),
        "igrave" => "\u{00EC}".into(),
        "iacute" => "\u{00ED}".into(),
        "icircumflex" => "\u{00EE}".into(),
        "idieresis" => "\u{00EF}".into(),
        "eth" => "\u{00F0}".into(),
        "ntilde" => "\u{00F1}".into(),
        "ograve" => "\u{00F2}".into(),
        "oacute" => "\u{00F3}".into(),
        "ocircumflex" => "\u{00F4}".into(),
        "otilde" => "\u{00F5}".into(),
        "odieresis" => "\u{00F6}".into(),
        "oslash" => "\u{00F8}".into(),
        "ugrave" => "\u{00F9}".into(),
        "uacute" => "\u{00FA}".into(),
        "ucircumflex" => "\u{00FB}".into(),
        "udieresis" => "\u{00FC}".into(),
        "yacute" => "\u{00FD}".into(),
        "thorn" => "\u{00FE}".into(),
        "ydieresis" => "\u{00FF}".into(),

        // Greek uppercase
        "Alpha" => "\u{0391}".into(),
        "Beta" => "\u{0392}".into(),
        "Gamma" => "\u{0393}".into(),
        "Delta" => "\u{0394}".into(),
        "Epsilon" => "\u{0395}".into(),
        "Zeta" => "\u{0396}".into(),
        "Eta" => "\u{0397}".into(),
        "Theta" => "\u{0398}".into(),
        "Iota" => "\u{0399}".into(),
        "Kappa" => "\u{039A}".into(),
        "Lambda" => "\u{039B}".into(),
        "Mu" => "\u{039C}".into(),
        "Nu" => "\u{039D}".into(),
        "Xi" => "\u{039E}".into(),
        "Omicron" => "\u{039F}".into(),
        "Pi" => "\u{03A0}".into(),
        "Rho" => "\u{03A1}".into(),
        "Sigma" => "\u{03A3}".into(),
        "Tau" => "\u{03A4}".into(),
        "Upsilon" => "\u{03A5}".into(),
        "Phi" => "\u{03A6}".into(),
        "Chi" => "\u{03A7}".into(),
        "Psi" => "\u{03A8}".into(),
        "Omega" => "\u{03A9}".into(),

        // Greek lowercase
        "alpha" => "\u{03B1}".into(),
        "beta" => "\u{03B2}".into(),
        "gamma" => "\u{03B3}".into(),
        "delta" => "\u{03B4}".into(),
        "epsilon" | "varepsilon" => "\u{03B5}".into(),
        "zeta" => "\u{03B6}".into(),
        "eta" => "\u{03B7}".into(),
        "theta" => "\u{03B8}".into(),
        "iota" => "\u{03B9}".into(),
        "kappa" => "\u{03BA}".into(),
        "lambda" => "\u{03BB}".into(),
        "mu" => "\u{03BC}".into(),
        "nu" => "\u{03BD}".into(),
        "xi" => "\u{03BE}".into(),
        "omicron" => "\u{03BF}".into(),
        "pi" => "\u{03C0}".into(),
        "rho" => "\u{03C1}".into(),
        "sigma" => "\u{03C3}".into(),
        "varsigma" | "sigmafinal" => "\u{03C2}".into(),
        "tau" => "\u{03C4}".into(),
        "upsilon" => "\u{03C5}".into(),
        "phi" | "varphi" => "\u{03C6}".into(),
        "chi" => "\u{03C7}".into(),
        "psi" => "\u{03C8}".into(),
        "omega" => "\u{03C9}".into(),
        "vartheta" => "\u{03D1}".into(),
        "varpi" => "\u{03D6}".into(),

        // Math / symbol
        "multiply" => "\u{00D7}".into(),
        "divide" => "\u{00F7}".into(),
        "plusminus" => "\u{00B1}".into(),
        "minusmath" => "\u{2212}".into(),
        "notequal" => "\u{2260}".into(),
        "lessequal" | "leq" => "\u{2264}".into(),
        "greaterequal" | "geq" => "\u{2265}".into(),
        "infinity" => "\u{221E}".into(),
        "summation" => "\u{2211}".into(),
        "product" => "\u{220F}".into(),
        "integral" => "\u{222B}".into(),
        "radical" | "sqrt" => "\u{221A}".into(),
        "approxequal" | "approx" => "\u{2248}".into(),
        "nabla" | "gradient" => "\u{2207}".into(),
        "partial" | "partialdiff" => "\u{2202}".into(),
        "element" | "in" => "\u{2208}".into(),
        "notelement" | "notin" => "\u{2209}".into(),
        "propersubset" | "subset" => "\u{2282}".into(),
        "propersuperset" | "superset" => "\u{2283}".into(),
        "reflexsubset" | "subseteq" => "\u{2286}".into(),
        "reflexsuperset" | "supseteq" => "\u{2287}".into(),
        "union" | "cup" => "\u{222A}".into(),
        "intersection" | "cap" => "\u{2229}".into(),
        "emptyset" => "\u{2205}".into(),
        "forall" | "universal" => "\u{2200}".into(),
        "existential" | "exists" => "\u{2203}".into(),
        "logicaland" | "wedge" => "\u{2227}".into(),
        "logicalor" | "vee" => "\u{2228}".into(),
        "logicalnot" | "neg" => "\u{00AC}".into(),
        "therefore" => "\u{2234}".into(),
        "because" => "\u{2235}".into(),
        "equivalence" | "equiv" => "\u{2261}".into(),
        "proportional" | "propto" => "\u{221D}".into(),
        "perpendicular" | "perp" => "\u{22A5}".into(),
        "angle" => "\u{2220}".into(),
        "arrowleft" | "leftarrow" => "\u{2190}".into(),
        "arrowup" | "uparrow" => "\u{2191}".into(),
        "arrowright" | "rightarrow" => "\u{2192}".into(),
        "arrowdown" | "downarrow" => "\u{2193}".into(),
        "arrowboth" | "leftrightarrow" => "\u{2194}".into(),
        "Arrowleft" | "Leftarrow" => "\u{21D0}".into(),
        "Arrowright" | "Rightarrow" => "\u{21D2}".into(),
        "Arrowboth" | "Leftrightarrow" => "\u{21D4}".into(),
        "aleph" => "\u{2135}".into(),
        "Ifraktur" | "Im" => "\u{2111}".into(),
        "Rfraktur" | "Re" => "\u{211C}".into(),
        "weierstrass" | "wp" => "\u{2118}".into(),
        "circleplus" | "oplus" => "\u{2295}".into(),
        "circlemultiply" | "otimes" => "\u{2297}".into(),
        "dotmath" | "cdot" | "periodcentered" | "middot" => "\u{00B7}".into(),
        "times" => "\u{00D7}".into(),
        "star" => "\u{22C6}".into(),

        // Other Latin
        "Lslash" => "\u{0141}".into(),
        "lslash" => "\u{0142}".into(),
        "OE" => "\u{0152}".into(),
        "oe" => "\u{0153}".into(),
        "Scaron" => "\u{0160}".into(),
        "scaron" => "\u{0161}".into(),
        "Zcaron" => "\u{017D}".into(),
        "zcaron" => "\u{017E}".into(),
        "Ydieresis" => "\u{0178}".into(),
        "dotlessi" => "\u{0131}".into(),
        "circumflex" => "\u{02C6}".into(),
        "tilde" => "\u{02DC}".into(),
        "breve" => "\u{02D8}".into(),
        "dotaccent" => "\u{02D9}".into(),
        "ring" => "\u{02DA}".into(),
        "cedilla" => "\u{00B8}".into(),
        "hungarumlaut" => "\u{02DD}".into(),
        "ogonek" => "\u{02DB}".into(),
        "caron" => "\u{02C7}".into(),
        "macron" => "\u{00AF}".into(),
        "dieresis" => "\u{00A8}".into(),
        "acute" => "\u{00B4}".into(),

        // Shapes
        "blackdiamond" | "diamond" => "\u{25C6}".into(),
        "filledbox" | "blacksquare" => "\u{25A0}".into(),
        "filledcircle" | "blackcircle" => "\u{25CF}".into(),
        "opendiamond" => "\u{25C7}".into(),
        "openbox" | "square" => "\u{25A1}".into(),
        "circle" | "opencircle" => "\u{25CB}".into(),
        "triagup" => "\u{25B2}".into(),
        "triagdn" => "\u{25BC}".into(),

        // For "uniXXXX" format
        _ if name_str.starts_with("uni") && name_str.len() == 7 => {
            u32::from_str_radix(&name_str[3..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| name_str.into())
        }
        // For "uXXXX" format
        _ if name_str.starts_with('u')
            && name_str.len() >= 5
            && name_str[1..].chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            u32::from_str_radix(&name_str[1..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_else(|| name_str.into())
        }
        // Single-letter names match directly
        _ if name_str.len() == 1 => name_str.into(),
        _ => name_str.into(),
    }
}

/// Decode text bytes using the appropriate encoding for a font.
pub fn decode_text(
    data: &[u8],
    encoding: &Encoding,
    tounicode: Option<&super::cmap::ToUnicodeCMap>,
) -> String {
    if let Some(cmap) = tounicode {
        return cmap.decode(data);
    }
    encoding.decode_bytes(data)
}
