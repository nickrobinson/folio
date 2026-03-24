//! UniFFI bindings for the Folio PDF library.
//!
//! This crate exposes the Folio API to Swift, Kotlin, Python, and other
//! languages via Mozilla's UniFFI.

use folio_pdf::core::{ColorPt, Matrix2D, PdfDate, Point, Rect};

uniffi::include_scaffolding!("folio");

/// Return the Folio library version.
fn folio_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// --- UniFFI-compatible wrapper types ---
// UniFFI needs its own record types defined in the UDL.
// These conversion impls bridge between core types and UniFFI types.

/// UniFFI-compatible rectangle.
#[derive(Debug, Clone)]
pub struct FolioRect {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl From<Rect> for FolioRect {
    fn from(r: Rect) -> Self {
        Self {
            x1: r.x1,
            y1: r.y1,
            x2: r.x2,
            y2: r.y2,
        }
    }
}

impl From<FolioRect> for Rect {
    fn from(r: FolioRect) -> Self {
        Self::new(r.x1, r.y1, r.x2, r.y2)
    }
}

/// UniFFI-compatible point.
#[derive(Debug, Clone)]
pub struct FolioPoint {
    pub x: f64,
    pub y: f64,
}

impl From<Point> for FolioPoint {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

/// UniFFI-compatible matrix.
#[derive(Debug, Clone)]
pub struct FolioMatrix2D {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub h: f64,
    pub v: f64,
}

impl From<Matrix2D> for FolioMatrix2D {
    fn from(m: Matrix2D) -> Self {
        Self {
            a: m.a,
            b: m.b,
            c: m.c,
            d: m.d,
            h: m.h,
            v: m.v,
        }
    }
}

/// UniFFI-compatible color point.
#[derive(Debug, Clone)]
pub struct FolioColorPt {
    pub c0: f64,
    pub c1: f64,
    pub c2: f64,
    pub c3: f64,
}

impl From<ColorPt> for FolioColorPt {
    fn from(c: ColorPt) -> Self {
        Self {
            c0: c.c0,
            c1: c.c1,
            c2: c.c2,
            c3: c.c3,
        }
    }
}

/// UniFFI-compatible PDF date.
#[derive(Debug, Clone)]
pub struct FolioPdfDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub ut: String,
    pub ut_hour: u8,
    pub ut_minutes: u8,
}

impl From<PdfDate> for FolioPdfDate {
    fn from(d: PdfDate) -> Self {
        Self {
            year: d.year,
            month: d.month,
            day: d.day,
            hour: d.hour,
            minute: d.minute,
            second: d.second,
            ut: d.ut.to_string(),
            ut_hour: d.ut_hour,
            ut_minutes: d.ut_minutes,
        }
    }
}

/// UniFFI-compatible error type.
#[derive(Debug, thiserror::Error)]
pub enum FolioError {
    #[error("I/O error: {message}")]
    Io { message: String },
    #[error("Parse error at {offset}: {message}")]
    Parse { offset: u64, message: String },
    #[error("Invalid object: {message}")]
    InvalidObject { message: String },
    #[error("Encryption error: {message}")]
    Encryption { message: String },
    #[error("Signature error: {message}")]
    Signature { message: String },
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },
    #[error("Unsupported feature: {message}")]
    UnsupportedFeature { message: String },
    #[error("Oracle error: {message}")]
    Oracle { message: String },
}
