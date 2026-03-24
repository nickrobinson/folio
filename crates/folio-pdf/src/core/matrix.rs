//! 2D affine transformation matrix.
//!
//! Matches the PDF specification's transformation matrix:
//! ```text
//! | a  b  0 |
//! | c  d  0 |
//! | h  v  1 |
//! ```

use super::Point;

/// A 2D affine transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix2D {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub h: f64,
    pub v: f64,
}

impl Matrix2D {
    /// Create a new matrix with explicit components.
    pub fn new(a: f64, b: f64, c: f64, d: f64, h: f64, v: f64) -> Self {
        Self { a, b, c, d, h, v }
    }

    /// The identity matrix.
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    /// Create a translation matrix.
    pub fn translation(h: f64, v: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, h, v)
    }

    /// Create a scaling matrix.
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// Create a rotation matrix (angle in radians).
    pub fn rotation(angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }

    /// Multiply this matrix by another: self * other.
    /// This concatenates transformations: first `other` is applied, then `self`.
    pub fn multiply(&self, other: &Matrix2D) -> Matrix2D {
        Matrix2D {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            h: self.h * other.a + self.v * other.c + other.h,
            v: self.h * other.b + self.v * other.d + other.v,
        }
    }

    /// Concatenate: equivalent to `other * self` (apply self first, then other).
    pub fn concat(&self, other: &Matrix2D) -> Matrix2D {
        other.multiply(self)
    }

    /// Compute the determinant.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Compute the inverse matrix. Returns None if the matrix is singular.
    pub fn inverse(&self) -> Option<Matrix2D> {
        let det = self.determinant();
        if det.abs() < 1e-14 {
            return None;
        }
        let inv_det = 1.0 / det;
        Some(Matrix2D {
            a: self.d * inv_det,
            b: -self.b * inv_det,
            c: -self.c * inv_det,
            d: self.a * inv_det,
            h: (self.c * self.v - self.d * self.h) * inv_det,
            v: (self.b * self.h - self.a * self.v) * inv_det,
        })
    }

    /// Transform a point through this matrix.
    pub fn transform_point(&self, x: f64, y: f64) -> Point {
        Point {
            x: self.a * x + self.c * y + self.h,
            y: self.b * x + self.d * y + self.v,
        }
    }
}

impl Default for Matrix2D {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::ops::Mul for Matrix2D {
    type Output = Matrix2D;
    fn mul(self, rhs: Matrix2D) -> Matrix2D {
        self.multiply(&rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-10
    }

    #[test]
    fn test_identity() {
        let m = Matrix2D::identity();
        let p = m.transform_point(3.0, 4.0);
        assert!(approx_eq(p.x, 3.0));
        assert!(approx_eq(p.y, 4.0));
    }

    #[test]
    fn test_translation() {
        let m = Matrix2D::translation(10.0, 20.0);
        let p = m.transform_point(5.0, 5.0);
        assert!(approx_eq(p.x, 15.0));
        assert!(approx_eq(p.y, 25.0));
    }

    #[test]
    fn test_scale() {
        let m = Matrix2D::scale(2.0, 3.0);
        let p = m.transform_point(5.0, 5.0);
        assert!(approx_eq(p.x, 10.0));
        assert!(approx_eq(p.y, 15.0));
    }

    #[test]
    fn test_inverse() {
        let m = Matrix2D::new(2.0, 1.0, 1.0, 3.0, 5.0, 7.0);
        let inv = m.inverse().unwrap();
        let product = m.multiply(&inv);
        assert!(approx_eq(product.a, 1.0));
        assert!(approx_eq(product.b, 0.0));
        assert!(approx_eq(product.c, 0.0));
        assert!(approx_eq(product.d, 1.0));
        assert!(approx_eq(product.h, 0.0));
        assert!(approx_eq(product.v, 0.0));
    }

    #[test]
    fn test_singular_matrix() {
        let m = Matrix2D::new(1.0, 2.0, 2.0, 4.0, 0.0, 0.0);
        assert!(m.inverse().is_none());
    }

    #[test]
    fn test_rotation() {
        let m = Matrix2D::rotation(std::f64::consts::FRAC_PI_2); // 90 degrees
        let p = m.transform_point(1.0, 0.0);
        assert!(approx_eq(p.x, 0.0));
        assert!(approx_eq(p.y, 1.0));
    }

    #[test]
    fn test_multiply_associativity() {
        let a = Matrix2D::translation(1.0, 2.0);
        let b = Matrix2D::scale(2.0, 3.0);
        let c = Matrix2D::rotation(0.5);
        let ab_c = (a * b) * c;
        let a_bc = a * (b * c);
        assert!(approx_eq(ab_c.a, a_bc.a));
        assert!(approx_eq(ab_c.b, a_bc.b));
        assert!(approx_eq(ab_c.c, a_bc.c));
        assert!(approx_eq(ab_c.d, a_bc.d));
        assert!(approx_eq(ab_c.h, a_bc.h));
        assert!(approx_eq(ab_c.v, a_bc.v));
    }
}
