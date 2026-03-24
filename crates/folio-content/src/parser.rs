//! Content stream parser — converts raw PDF content stream bytes into ContentOp sequence.

use crate::ops::{ContentOp, TextOp};
use folio_core::{Matrix2D, Result};
use folio_cos::PdfObject;
use folio_cos::parser::parse_object;
use folio_cos::tokenizer::{Token, Tokenizer};

/// Parse a content stream into a sequence of operations.
pub fn parse_content_stream(data: &[u8]) -> Result<Vec<ContentOp>> {
    let mut tokenizer = Tokenizer::new_at(data, 0);
    let mut ops = Vec::new();
    let mut operand_stack: Vec<PdfObject> = Vec::new();

    loop {
        tokenizer.skip_whitespace_and_comments();
        if tokenizer.is_eof() {
            break;
        }

        // Check for inline image (BI keyword)
        let pos = tokenizer.pos();
        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"BI" {
            // Check it's actually the keyword (followed by whitespace)
            if pos + 2 >= data.len() || is_whitespace_or_delimiter(data[pos + 2]) {
                tokenizer.set_pos(pos + 2);
                let op = parse_inline_image(&mut tokenizer)?;
                ops.push(op);
                operand_stack.clear();
                continue;
            }
        }

        let token = match tokenizer.next_token()? {
            Some(t) => t,
            None => break,
        };

        match token {
            Token::Integer(_)
            | Token::Real(_)
            | Token::LiteralString(_)
            | Token::HexString(_)
            | Token::Name(_)
            | Token::ArrayBegin => {
                // It's an operand — push onto stack
                tokenizer.set_pos(pos);
                match parse_object(&mut tokenizer)? {
                    Some(obj) => operand_stack.push(obj),
                    None => {}
                }
            }
            Token::Keyword(ref kw) => {
                let op = build_op(kw, &operand_stack);
                ops.push(op);
                operand_stack.clear();
            }
            Token::DictBegin => {
                // Dict as operand (used in BDC, DP)
                tokenizer.set_pos(pos);
                if let Some(obj) = parse_object(&mut tokenizer)? {
                    operand_stack.push(obj);
                }
            }
            _ => {
                // Unexpected token — skip
            }
        }
    }

    Ok(ops)
}

fn is_whitespace_or_delimiter(b: u8) -> bool {
    folio_cos::tokenizer::is_whitespace(b) || folio_cos::tokenizer::is_delimiter(b)
}

/// Build a ContentOp from an operator keyword and its operands.
fn build_op(operator: &[u8], operands: &[PdfObject]) -> ContentOp {
    match operator {
        // Graphics state
        b"q" => ContentOp::SaveState,
        b"Q" => ContentOp::RestoreState,
        b"cm" if operands.len() >= 6 => ContentOp::ConcatMatrix(Matrix2D::new(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
            f(operands, 4),
            f(operands, 5),
        )),
        b"w" => ContentOp::SetLineWidth(f(operands, 0)),
        b"J" => ContentOp::SetLineCap(i(operands, 0)),
        b"j" => ContentOp::SetLineJoin(i(operands, 0)),
        b"M" => ContentOp::SetMiterLimit(f(operands, 0)),
        b"d" => {
            let arr = operands
                .first()
                .and_then(|o| o.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let phase = f(operands, 1);
            ContentOp::SetDashPattern(arr, phase)
        }
        b"ri" => ContentOp::SetRenderingIntent(n(operands, 0)),
        b"i" => ContentOp::SetFlatness(f(operands, 0)),
        b"gs" => ContentOp::SetExtGState(n(operands, 0)),

        // Path construction
        b"m" => ContentOp::MoveTo(f(operands, 0), f(operands, 1)),
        b"l" => ContentOp::LineTo(f(operands, 0), f(operands, 1)),
        b"c" => ContentOp::CurveTo(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
            f(operands, 4),
            f(operands, 5),
        ),
        b"v" => ContentOp::CurveToInitial(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
        ),
        b"y" => ContentOp::CurveToFinal(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
        ),
        b"h" => ContentOp::ClosePath,
        b"re" => ContentOp::Rectangle(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
        ),

        // Path painting
        b"S" => ContentOp::Stroke,
        b"s" => ContentOp::CloseAndStroke,
        b"f" | b"F" => ContentOp::Fill,
        b"f*" => ContentOp::FillEvenOdd,
        b"B" => ContentOp::FillAndStroke,
        b"B*" => ContentOp::FillAndStrokeEvenOdd,
        b"b" => ContentOp::CloseFillAndStroke,
        b"b*" => ContentOp::CloseFillAndStrokeEvenOdd,
        b"n" => ContentOp::EndPath,

        // Clipping
        b"W" => ContentOp::Clip,
        b"W*" => ContentOp::ClipEvenOdd,

        // Text
        b"BT" => ContentOp::BeginText,
        b"ET" => ContentOp::EndText,
        b"Tc" => ContentOp::SetCharSpacing(f(operands, 0)),
        b"Tw" => ContentOp::SetWordSpacing(f(operands, 0)),
        b"Tz" => ContentOp::SetHorizScaling(f(operands, 0)),
        b"TL" => ContentOp::SetTextLeading(f(operands, 0)),
        b"Tf" => ContentOp::SetFont(n(operands, 0), f(operands, 1)),
        b"Tr" => ContentOp::SetTextRenderMode(i(operands, 0)),
        b"Ts" => ContentOp::SetTextRise(f(operands, 0)),
        b"Td" => ContentOp::MoveTextPos(f(operands, 0), f(operands, 1)),
        b"TD" => ContentOp::MoveTextPosSetLeading(f(operands, 0), f(operands, 1)),
        b"Tm" if operands.len() >= 6 => ContentOp::SetTextMatrix(Matrix2D::new(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
            f(operands, 4),
            f(operands, 5),
        )),
        b"T*" => ContentOp::NextLine,
        b"Tj" => ContentOp::ShowText(s(operands, 0)),
        b"TJ" => {
            let items = operands
                .first()
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|item| match item {
                            PdfObject::Str(s) => TextOp::Text(s.clone()),
                            PdfObject::Integer(n) => TextOp::Adjustment(*n as f64),
                            PdfObject::Real(n) => TextOp::Adjustment(*n),
                            _ => TextOp::Adjustment(0.0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            ContentOp::ShowTextAdjusted(items)
        }
        b"'" => ContentOp::NextLineShowText(s(operands, 0)),
        b"\"" => {
            ContentOp::SetSpacingNextLineShowText(f(operands, 0), f(operands, 1), s(operands, 2))
        }

        // Color
        b"CS" => ContentOp::SetStrokeColorSpace(n(operands, 0)),
        b"cs" => ContentOp::SetFillColorSpace(n(operands, 0)),
        b"SC" | b"SCN" => {
            ContentOp::SetStrokeColor(operands.iter().filter_map(|o| o.as_f64()).collect())
        }
        b"sc" | b"scn" => {
            ContentOp::SetFillColor(operands.iter().filter_map(|o| o.as_f64()).collect())
        }
        b"G" => ContentOp::SetStrokeGray(f(operands, 0)),
        b"g" => ContentOp::SetFillGray(f(operands, 0)),
        b"RG" => ContentOp::SetStrokeRGB(f(operands, 0), f(operands, 1), f(operands, 2)),
        b"rg" => ContentOp::SetFillRGB(f(operands, 0), f(operands, 1), f(operands, 2)),
        b"K" => ContentOp::SetStrokeCMYK(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
        ),
        b"k" => ContentOp::SetFillCMYK(
            f(operands, 0),
            f(operands, 1),
            f(operands, 2),
            f(operands, 3),
        ),

        // XObject / Shading
        b"Do" => ContentOp::PaintXObject(n(operands, 0)),
        b"sh" => ContentOp::PaintShading(n(operands, 0)),

        // Marked content
        b"MP" => ContentOp::MarkedContentPoint(n(operands, 0)),
        b"DP" => ContentOp::MarkedContentPointProperties(
            n(operands, 0),
            operands.get(1).cloned().unwrap_or(PdfObject::Null),
        ),
        b"BMC" => ContentOp::BeginMarkedContent(n(operands, 0)),
        b"BDC" => ContentOp::BeginMarkedContentProperties(
            n(operands, 0),
            operands.get(1).cloned().unwrap_or(PdfObject::Null),
        ),
        b"EMC" => ContentOp::EndMarkedContent,

        // Compatibility
        b"BX" => ContentOp::BeginCompat,
        b"EX" => ContentOp::EndCompat,

        // Unknown
        _ => ContentOp::Unknown(operator.to_vec(), operands.to_vec()),
    }
}

/// Parse an inline image (after BI keyword has been consumed).
fn parse_inline_image(tokenizer: &mut Tokenizer) -> Result<ContentOp> {
    tokenizer.skip_whitespace_and_comments();

    // Parse key-value pairs until ID keyword
    let mut dict = Vec::new();
    loop {
        tokenizer.skip_whitespace_and_comments();
        if tokenizer.is_eof() {
            break;
        }

        // Check for ID keyword
        let pos = tokenizer.pos();
        let data = tokenizer.data();
        if pos + 2 <= data.len() && &data[pos..pos + 2] == b"ID" {
            tokenizer.set_pos(pos + 2);
            // Skip single whitespace byte after ID
            if !tokenizer.is_eof() {
                tokenizer.set_pos(tokenizer.pos() + 1);
            }
            break;
        }

        match tokenizer.next_token()? {
            Some(Token::Name(key)) => {
                // Expand abbreviated key names
                let full_key = expand_inline_image_key(&key);
                match parse_object(tokenizer)? {
                    Some(val) => dict.push((full_key, val)),
                    None => break,
                }
            }
            _ => break,
        }
    }

    // Read image data until EI
    let start = tokenizer.pos();
    let data = tokenizer.data();
    let mut end = start;

    // Search for EI preceded by whitespace
    while end < data.len() {
        if end + 2 < data.len()
            && data[end] == b'E'
            && data[end + 1] == b'I'
            && (end == start || is_whitespace_byte(data[end - 1]))
            && (end + 2 >= data.len() || is_whitespace_or_delimiter(data[end + 2]))
        {
            break;
        }
        end += 1;
    }

    // Trim trailing whitespace from image data
    let mut img_end = end;
    while img_end > start && is_whitespace_byte(data[img_end - 1]) {
        img_end -= 1;
    }

    let image_data = data[start..img_end].to_vec();
    tokenizer.set_pos(end + 2); // Skip past EI

    Ok(ContentOp::InlineImage {
        dict,
        data: image_data,
    })
}

fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x00')
}

/// Expand abbreviated inline image key names to full names.
fn expand_inline_image_key(key: &[u8]) -> Vec<u8> {
    match key {
        b"BPC" => b"BitsPerComponent".to_vec(),
        b"CS" => b"ColorSpace".to_vec(),
        b"D" => b"Decode".to_vec(),
        b"DP" => b"DecodeParms".to_vec(),
        b"F" => b"Filter".to_vec(),
        b"H" => b"Height".to_vec(),
        b"IM" => b"ImageMask".to_vec(),
        b"I" => b"Interpolate".to_vec(),
        b"W" => b"Width".to_vec(),
        _ => key.to_vec(),
    }
}

// --- Operand helpers ---
fn f(ops: &[PdfObject], idx: usize) -> f64 {
    ops.get(idx).and_then(|o| o.as_f64()).unwrap_or(0.0)
}
fn i(ops: &[PdfObject], idx: usize) -> i32 {
    ops.get(idx).and_then(|o| o.as_i64()).unwrap_or(0) as i32
}
fn n(ops: &[PdfObject], idx: usize) -> Vec<u8> {
    ops.get(idx)
        .and_then(|o| o.as_name())
        .unwrap_or(b"")
        .to_vec()
}
fn s(ops: &[PdfObject], idx: usize) -> Vec<u8> {
    ops.get(idx)
        .and_then(|o| o.as_str())
        .unwrap_or(b"")
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_ops() {
        let data = b"q 1 0 0 1 100 200 cm Q";
        let ops = parse_content_stream(data).unwrap();
        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], ContentOp::SaveState));
        assert!(matches!(ops[1], ContentOp::ConcatMatrix(_)));
        assert!(matches!(ops[2], ContentOp::RestoreState));
    }

    #[test]
    fn test_text_ops() {
        let data = b"BT /F1 12 Tf 100 700 Td (Hello World) Tj ET";
        let ops = parse_content_stream(data).unwrap();
        assert!(matches!(ops[0], ContentOp::BeginText));
        assert!(matches!(ops[1], ContentOp::SetFont(..)));
        assert!(matches!(ops[2], ContentOp::MoveTextPos(..)));
        assert!(matches!(ops[3], ContentOp::ShowText(..)));
        assert!(matches!(ops[4], ContentOp::EndText));

        if let ContentOp::SetFont(ref name, size) = ops[1] {
            assert_eq!(name, b"F1");
            assert_eq!(size, 12.0);
        }
        if let ContentOp::ShowText(ref text) = ops[3] {
            assert_eq!(text, b"Hello World");
        }
    }

    #[test]
    fn test_path_ops() {
        let data = b"100 200 m 300 400 l 100 200 300 400 500 600 c h S";
        let ops = parse_content_stream(data).unwrap();
        assert!(matches!(ops[0], ContentOp::MoveTo(100.0, 200.0)));
        assert!(matches!(ops[1], ContentOp::LineTo(300.0, 400.0)));
        assert!(matches!(ops[2], ContentOp::CurveTo(..)));
        assert!(matches!(ops[3], ContentOp::ClosePath));
        assert!(matches!(ops[4], ContentOp::Stroke));
    }

    #[test]
    fn test_color_ops() {
        let data = b"1 0 0 RG 0.5 g";
        let ops = parse_content_stream(data).unwrap();
        assert!(matches!(ops[0], ContentOp::SetStrokeRGB(1.0, 0.0, 0.0)));
        assert!(matches!(ops[1], ContentOp::SetFillGray(..)));
    }

    #[test]
    fn test_tj_array() {
        let data = b"[(Hello ) -100 (World)] TJ";
        let ops = parse_content_stream(data).unwrap();
        assert_eq!(ops.len(), 1);
        if let ContentOp::ShowTextAdjusted(ref items) = ops[0] {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], TextOp::Text(ref t) if t == b"Hello "));
            assert!(matches!(items[1], TextOp::Adjustment(-100.0)));
            assert!(matches!(items[2], TextOp::Text(ref t) if t == b"World"));
        } else {
            panic!("Expected ShowTextAdjusted");
        }
    }

    #[test]
    fn test_marked_content() {
        let data = b"/Span BMC (text) Tj EMC";
        let ops = parse_content_stream(data).unwrap();
        assert!(matches!(ops[0], ContentOp::BeginMarkedContent(..)));
        assert!(matches!(ops[1], ContentOp::ShowText(..)));
        assert!(matches!(ops[2], ContentOp::EndMarkedContent));
    }

    #[test]
    fn test_xobject() {
        let data = b"/Im0 Do";
        let ops = parse_content_stream(data).unwrap();
        assert_eq!(ops.len(), 1);
        if let ContentOp::PaintXObject(ref name) = ops[0] {
            assert_eq!(name, b"Im0");
        }
    }
}
