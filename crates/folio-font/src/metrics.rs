//! Standard 14 font metrics.
//!
//! The PDF spec defines 14 standard fonts that every PDF viewer must support.
//! These fonts don't need to be embedded — their metrics are built-in.

/// Built-in metrics for the standard 14 fonts.
///
/// Each entry is (font_name, default_width).
/// Widths are in units of 1/1000 of a text unit.
pub const STANDARD_14_METRICS: &[(&str, f64)] = &[
    ("Courier", 600.0),
    ("Courier-Bold", 600.0),
    ("Courier-Oblique", 600.0),
    ("Courier-BoldOblique", 600.0),
    ("Helvetica", 278.0),
    ("Helvetica-Bold", 278.0),
    ("Helvetica-Oblique", 278.0),
    ("Helvetica-BoldOblique", 278.0),
    ("Times-Roman", 250.0),
    ("Times-Bold", 250.0),
    ("Times-Italic", 250.0),
    ("Times-BoldItalic", 250.0),
    ("Symbol", 250.0),
    ("ZapfDingbats", 278.0),
];

/// Check if a font name is one of the standard 14.
pub fn is_standard_14(name: &str) -> bool {
    STANDARD_14_METRICS.iter().any(|(n, _)| *n == name)
}

/// Get the default width for a standard 14 font.
pub fn standard_14_default_width(name: &str) -> Option<f64> {
    STANDARD_14_METRICS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, w)| *w)
}
