//! PDF rectangle type (lower-left x, lower-left y, upper-right x, upper-right y).

/// A rectangle defined by two corner points.
///
/// In PDF coordinate space, (x1, y1) is typically the lower-left corner
/// and (x2, y2) is the upper-right corner, though this is not enforced
/// until `normalize()` is called.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Rect {
    /// Create a new rectangle.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Create a zero-sized rectangle at the origin.
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Width of the rectangle (may be negative if not normalized).
    pub fn width(&self) -> f64 {
        self.x2 - self.x1
    }

    /// Height of the rectangle (may be negative if not normalized).
    pub fn height(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Normalize so that (x1,y1) is lower-left and (x2,y2) is upper-right.
    pub fn normalize(&mut self) {
        if self.x1 > self.x2 {
            std::mem::swap(&mut self.x1, &mut self.x2);
        }
        if self.y1 > self.y2 {
            std::mem::swap(&mut self.y1, &mut self.y2);
        }
    }

    /// Return a normalized copy.
    pub fn normalized(&self) -> Self {
        let mut r = *self;
        r.normalize();
        r
    }

    /// Returns true if the point (x, y) is inside this rectangle.
    /// The rectangle should be normalized first for correct results.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x1 && x <= self.x2 && y >= self.y1 && y <= self.y2
    }

    /// Compute the intersection of two rectangles.
    /// Returns None if they do not intersect.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x1 = self.x1.max(other.x1);
        let y1 = self.y1.max(other.y1);
        let x2 = self.x2.min(other.x2);
        let y2 = self.y2.min(other.y2);
        if x1 <= x2 && y1 <= y2 {
            Some(Rect { x1, y1, x2, y2 })
        } else {
            None
        }
    }

    /// Compute the union (bounding box) of two rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    /// Inflate the rectangle by the given amounts on each side.
    pub fn inflate(&mut self, dx: f64, dy: f64) {
        self.x1 -= dx;
        self.y1 -= dy;
        self.x2 += dx;
        self.y2 += dy;
    }

    /// Inflate and return a new rectangle.
    pub fn inflated(&self, dx: f64, dy: f64) -> Self {
        let mut r = *self;
        r.inflate(dx, dy);
        r
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions() {
        let r = Rect::new(10.0, 20.0, 110.0, 70.0);
        assert_eq!(r.width(), 100.0);
        assert_eq!(r.height(), 50.0);
    }

    #[test]
    fn test_normalize() {
        let r = Rect::new(100.0, 200.0, 10.0, 20.0);
        let n = r.normalized();
        assert_eq!(n.x1, 10.0);
        assert_eq!(n.y1, 20.0);
        assert_eq!(n.x2, 100.0);
        assert_eq!(n.y2, 200.0);
    }

    #[test]
    fn test_contains() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(50.0, 50.0));
        assert!(!r.contains(150.0, 50.0));
        assert!(r.contains(0.0, 0.0)); // boundary
        assert!(r.contains(100.0, 100.0)); // boundary
    }

    #[test]
    fn test_intersect() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 150.0, 150.0);
        let i = a.intersect(&b).unwrap();
        assert_eq!(i, Rect::new(50.0, 50.0, 100.0, 100.0));

        let c = Rect::new(200.0, 200.0, 300.0, 300.0);
        assert!(a.intersect(&c).is_none());
    }

    #[test]
    fn test_union() {
        let a = Rect::new(10.0, 10.0, 50.0, 50.0);
        let b = Rect::new(30.0, 30.0, 80.0, 80.0);
        let u = a.union(&b);
        assert_eq!(u, Rect::new(10.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn test_inflate() {
        let r = Rect::new(10.0, 10.0, 50.0, 50.0);
        let i = r.inflated(5.0, 5.0);
        assert_eq!(i, Rect::new(5.0, 5.0, 55.0, 55.0));
    }
}
