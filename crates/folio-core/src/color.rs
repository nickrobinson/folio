//! Color point type (up to 4 components).

/// A color value with up to 4 components.
///
/// Interpretation depends on the color space:
/// - DeviceGray: c0 = gray level
/// - DeviceRGB: c0 = R, c1 = G, c2 = B
/// - DeviceCMYK: c0 = C, c1 = M, c2 = Y, c3 = K
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColorPt {
    pub c0: f64,
    pub c1: f64,
    pub c2: f64,
    pub c3: f64,
}

impl ColorPt {
    pub fn new(c0: f64, c1: f64, c2: f64, c3: f64) -> Self {
        Self { c0, c1, c2, c3 }
    }

    pub fn gray(g: f64) -> Self {
        Self::new(g, 0.0, 0.0, 0.0)
    }

    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self::new(r, g, b, 0.0)
    }

    pub fn cmyk(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self::new(c, m, y, k)
    }
}
