//! Specialized annotation type helpers.
//!
//! Provides convenience accessors for type-specific annotation properties.

use crate::annot::Annot;
use folio_core::{Point, QuadPoint};
use folio_cos::PdfObject;

/// Get highlight/underline/strikeout/squiggly quad points.
pub fn get_quad_points(annot: &Annot) -> Vec<QuadPoint> {
    let arr = match annot.dict().get(b"QuadPoints".as_slice()) {
        Some(PdfObject::Array(a)) => a,
        _ => return Vec::new(),
    };

    let mut quads = Vec::new();
    let mut i = 0;
    while i + 7 < arr.len() {
        if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x3), Some(y3), Some(x4), Some(y4)) = (
            arr[i].as_f64(),
            arr[i + 1].as_f64(),
            arr[i + 2].as_f64(),
            arr[i + 3].as_f64(),
            arr[i + 4].as_f64(),
            arr[i + 5].as_f64(),
            arr[i + 6].as_f64(),
            arr[i + 7].as_f64(),
        ) {
            quads.push(QuadPoint::new(
                Point::new(x1, y1),
                Point::new(x2, y2),
                Point::new(x3, y3),
                Point::new(x4, y4),
            ));
        }
        i += 8;
    }
    quads
}

/// Get line annotation endpoints.
pub fn get_line_endpoints(annot: &Annot) -> Option<(Point, Point)> {
    let arr = annot.dict().get(b"L".as_slice())?.as_array()?;
    if arr.len() >= 4 {
        Some((
            Point::new(arr[0].as_f64()?, arr[1].as_f64()?),
            Point::new(arr[2].as_f64()?, arr[3].as_f64()?),
        ))
    } else {
        None
    }
}

/// Get ink annotation ink lists (array of stroke paths).
pub fn get_ink_lists(annot: &Annot) -> Vec<Vec<Point>> {
    let ink_list = match annot.dict().get(b"InkList".as_slice()) {
        Some(PdfObject::Array(a)) => a,
        _ => return Vec::new(),
    };

    ink_list
        .iter()
        .filter_map(|stroke| {
            let arr = stroke.as_array()?;
            let mut points = Vec::new();
            let mut i = 0;
            while i + 1 < arr.len() {
                if let (Some(x), Some(y)) = (arr[i].as_f64(), arr[i + 1].as_f64()) {
                    points.push(Point::new(x, y));
                }
                i += 2;
            }
            Some(points)
        })
        .collect()
}

/// Get polygon/polyline vertices.
pub fn get_vertices(annot: &Annot) -> Vec<Point> {
    let arr = match annot.dict().get(b"Vertices".as_slice()) {
        Some(PdfObject::Array(a)) => a,
        _ => return Vec::new(),
    };

    let mut points = Vec::new();
    let mut i = 0;
    while i + 1 < arr.len() {
        if let (Some(x), Some(y)) = (arr[i].as_f64(), arr[i + 1].as_f64()) {
            points.push(Point::new(x, y));
        }
        i += 2;
    }
    points
}

/// Get the URI from a Link annotation's action.
pub fn get_link_uri(annot: &Annot) -> Option<String> {
    let action = annot.dict().get(b"A".as_slice())?;
    let action_dict = action.as_dict()?;
    let s_type = action_dict.get(b"S".as_slice())?.as_name()?;
    if s_type == b"URI" {
        action_dict
            .get(b"URI".as_slice())?
            .as_str()
            .map(|s| String::from_utf8_lossy(s).into_owned())
    } else {
        None
    }
}
