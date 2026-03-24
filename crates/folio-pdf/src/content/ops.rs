//! PDF content stream operator definitions.
//!
//! All ~70 PDF content stream operators are represented as enum variants.

use crate::core::Matrix2D;
use crate::cos::PdfObject;

/// A parsed content stream operator with its operands.
#[derive(Debug, Clone)]
pub enum ContentOp {
    // --- Graphics State ---
    /// `q` — Save graphics state
    SaveState,
    /// `Q` — Restore graphics state
    RestoreState,
    /// `cm` — Concatenate matrix to CTM
    ConcatMatrix(Matrix2D),
    /// `w` — Set line width
    SetLineWidth(f64),
    /// `J` — Set line cap style (0=butt, 1=round, 2=square)
    SetLineCap(i32),
    /// `j` — Set line join style (0=miter, 1=round, 2=bevel)
    SetLineJoin(i32),
    /// `M` — Set miter limit
    SetMiterLimit(f64),
    /// `d` — Set dash pattern [array, phase]
    SetDashPattern(Vec<f64>, f64),
    /// `ri` — Set rendering intent
    SetRenderingIntent(Vec<u8>),
    /// `i` — Set flatness tolerance
    SetFlatness(f64),
    /// `gs` — Set parameters from graphics state parameter dict
    SetExtGState(Vec<u8>),

    // --- Path Construction ---
    /// `m` — Move to (x, y)
    MoveTo(f64, f64),
    /// `l` — Line to (x, y)
    LineTo(f64, f64),
    /// `c` — Cubic Bézier curve (x1, y1, x2, y2, x3, y3)
    CurveTo(f64, f64, f64, f64, f64, f64),
    /// `v` — Cubic Bézier with first control point = current point
    CurveToInitial(f64, f64, f64, f64),
    /// `y` — Cubic Bézier with last control point = final point
    CurveToFinal(f64, f64, f64, f64),
    /// `h` — Close subpath
    ClosePath,
    /// `re` — Rectangle (x, y, w, h)
    Rectangle(f64, f64, f64, f64),

    // --- Path Painting ---
    /// `S` — Stroke path
    Stroke,
    /// `s` — Close and stroke path
    CloseAndStroke,
    /// `f` or `F` — Fill path (non-zero winding)
    Fill,
    /// `f*` — Fill path (even-odd rule)
    FillEvenOdd,
    /// `B` — Fill and stroke (non-zero winding)
    FillAndStroke,
    /// `B*` — Fill and stroke (even-odd rule)
    FillAndStrokeEvenOdd,
    /// `b` — Close, fill, and stroke (non-zero winding)
    CloseFillAndStroke,
    /// `b*` — Close, fill, and stroke (even-odd rule)
    CloseFillAndStrokeEvenOdd,
    /// `n` — End path without filling or stroking (used for clipping)
    EndPath,

    // --- Clipping ---
    /// `W` — Set clipping path (non-zero winding)
    Clip,
    /// `W*` — Set clipping path (even-odd rule)
    ClipEvenOdd,

    // --- Text Objects ---
    /// `BT` — Begin text object
    BeginText,
    /// `ET` — End text object
    EndText,

    // --- Text State ---
    /// `Tc` — Set character spacing
    SetCharSpacing(f64),
    /// `Tw` — Set word spacing
    SetWordSpacing(f64),
    /// `Tz` — Set horizontal scaling (percent)
    SetHorizScaling(f64),
    /// `TL` — Set text leading
    SetTextLeading(f64),
    /// `Tf` — Set font and size (font_name, size)
    SetFont(Vec<u8>, f64),
    /// `Tr` — Set text rendering mode
    SetTextRenderMode(i32),
    /// `Ts` — Set text rise
    SetTextRise(f64),

    // --- Text Positioning ---
    /// `Td` — Move text position (tx, ty)
    MoveTextPos(f64, f64),
    /// `TD` — Move text position and set leading (tx, ty)
    MoveTextPosSetLeading(f64, f64),
    /// `Tm` — Set text matrix
    SetTextMatrix(Matrix2D),
    /// `T*` — Move to start of next line
    NextLine,

    // --- Text Showing ---
    /// `Tj` — Show text string
    ShowText(Vec<u8>),
    /// `TJ` — Show text with positioning adjustments [(string, adjustment), ...]
    ShowTextAdjusted(Vec<TextOp>),
    /// `'` — Move to next line and show text
    NextLineShowText(Vec<u8>),
    /// `"` — Set word/char spacing, move to next line, show text
    SetSpacingNextLineShowText(f64, f64, Vec<u8>),

    // --- Color ---
    /// `CS` — Set stroke color space
    SetStrokeColorSpace(Vec<u8>),
    /// `cs` — Set fill color space
    SetFillColorSpace(Vec<u8>),
    /// `SC` or `SCN` — Set stroke color
    SetStrokeColor(Vec<f64>),
    /// `sc` or `scn` — Set fill color
    SetFillColor(Vec<f64>),
    /// `G` — Set stroke gray
    SetStrokeGray(f64),
    /// `g` — Set fill gray
    SetFillGray(f64),
    /// `RG` — Set stroke RGB
    SetStrokeRGB(f64, f64, f64),
    /// `rg` — Set fill RGB
    SetFillRGB(f64, f64, f64),
    /// `K` — Set stroke CMYK
    SetStrokeCMYK(f64, f64, f64, f64),
    /// `k` — Set fill CMYK
    SetFillCMYK(f64, f64, f64, f64),

    // --- XObject ---
    /// `Do` — Paint XObject (name)
    PaintXObject(Vec<u8>),

    // --- Shading ---
    /// `sh` — Paint shading pattern
    PaintShading(Vec<u8>),

    // --- Inline Image ---
    /// `BI`...`ID`...`EI` — Inline image
    InlineImage {
        dict: Vec<(Vec<u8>, PdfObject)>,
        data: Vec<u8>,
    },

    // --- Marked Content ---
    /// `MP` — Marked content point (tag)
    MarkedContentPoint(Vec<u8>),
    /// `DP` — Marked content point with properties (tag, properties)
    MarkedContentPointProperties(Vec<u8>, PdfObject),
    /// `BMC` — Begin marked content (tag)
    BeginMarkedContent(Vec<u8>),
    /// `BDC` — Begin marked content with properties (tag, properties)
    BeginMarkedContentProperties(Vec<u8>, PdfObject),
    /// `EMC` — End marked content
    EndMarkedContent,

    // --- Compatibility ---
    /// `BX` — Begin compatibility section
    BeginCompat,
    /// `EX` — End compatibility section
    EndCompat,

    /// Unknown operator
    Unknown(Vec<u8>, Vec<PdfObject>),
}

/// A text operation within a TJ array.
#[derive(Debug, Clone)]
pub enum TextOp {
    /// A text string to show.
    Text(Vec<u8>),
    /// A positioning adjustment (negative = move right, positive = move left).
    Adjustment(f64),
}

/// Path segment types for path data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    MoveTo,
    LineTo,
    CurveTo,
    ClosePath,
    Rectangle,
}
