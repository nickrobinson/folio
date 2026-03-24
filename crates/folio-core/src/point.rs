//! Point and QuadPoint types.

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A quadrilateral defined by four points.
/// Used for text highlights and other non-rectangular regions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QuadPoint {
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub p4: Point,
}

impl QuadPoint {
    pub fn new(p1: Point, p2: Point, p3: Point, p4: Point) -> Self {
        Self { p1, p2, p3, p4 }
    }
}
