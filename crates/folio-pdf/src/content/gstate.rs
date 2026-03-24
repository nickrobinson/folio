//! PDF Graphics State — tracks all visual properties during content stream processing.

use crate::core::Matrix2D;

/// The complete graphics state at any point during content stream processing.
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix.
    pub ctm: Matrix2D,
    /// Line width.
    pub line_width: f64,
    /// Line cap style (0=butt, 1=round, 2=square).
    pub line_cap: i32,
    /// Line join style (0=miter, 1=round, 2=bevel).
    pub line_join: i32,
    /// Miter limit.
    pub miter_limit: f64,
    /// Dash pattern.
    pub dash_array: Vec<f64>,
    /// Dash phase.
    pub dash_phase: f64,
    /// Fill color components.
    pub fill_color: Vec<f64>,
    /// Stroke color components.
    pub stroke_color: Vec<f64>,
    /// Fill color space name.
    pub fill_color_space: Vec<u8>,
    /// Stroke color space name.
    pub stroke_color_space: Vec<u8>,
    /// Character spacing.
    pub char_spacing: f64,
    /// Word spacing.
    pub word_spacing: f64,
    /// Horizontal scaling (percentage, default 100).
    pub horiz_scaling: f64,
    /// Text leading.
    pub text_leading: f64,
    /// Current font name (resource name, e.g., "F1").
    pub font_name: Vec<u8>,
    /// Current font size.
    pub font_size: f64,
    /// Text rendering mode (0-7).
    pub text_render_mode: i32,
    /// Text rise.
    pub text_rise: f64,
    /// Text matrix (set by Tm, modified by Td/TD/T*).
    pub text_matrix: Matrix2D,
    /// Text line matrix (set at start of line, used by T* and ').
    pub text_line_matrix: Matrix2D,
    /// Fill opacity (0.0 - 1.0).
    pub fill_opacity: f64,
    /// Stroke opacity (0.0 - 1.0).
    pub stroke_opacity: f64,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix2D::identity(),
            line_width: 1.0,
            line_cap: 0,
            line_join: 0,
            miter_limit: 10.0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            fill_color: vec![0.0],
            stroke_color: vec![0.0],
            fill_color_space: b"DeviceGray".to_vec(),
            stroke_color_space: b"DeviceGray".to_vec(),
            char_spacing: 0.0,
            word_spacing: 0.0,
            horiz_scaling: 100.0,
            text_leading: 0.0,
            font_name: Vec::new(),
            font_size: 0.0,
            text_render_mode: 0,
            text_rise: 0.0,
            text_matrix: Matrix2D::identity(),
            text_line_matrix: Matrix2D::identity(),
            fill_opacity: 1.0,
            stroke_opacity: 1.0,
        }
    }
}

/// A graphics state stack that supports save/restore (q/Q operators).
#[derive(Debug)]
pub struct GraphicsStateStack {
    current: GraphicsState,
    stack: Vec<GraphicsState>,
}

impl GraphicsStateStack {
    pub fn new() -> Self {
        Self {
            current: GraphicsState::default(),
            stack: Vec::new(),
        }
    }

    /// Get the current graphics state.
    pub fn current(&self) -> &GraphicsState {
        &self.current
    }

    /// Get a mutable reference to the current graphics state.
    pub fn current_mut(&mut self) -> &mut GraphicsState {
        &mut self.current
    }

    /// Save the current state (q operator).
    pub fn save(&mut self) {
        self.stack.push(self.current.clone());
    }

    /// Restore the previous state (Q operator).
    pub fn restore(&mut self) {
        if let Some(prev) = self.stack.pop() {
            self.current = prev;
        }
    }

    /// Stack depth (number of saved states).
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl Default for GraphicsStateStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_restore() {
        let mut stack = GraphicsStateStack::new();
        stack.current_mut().line_width = 5.0;
        stack.save();
        stack.current_mut().line_width = 10.0;
        assert_eq!(stack.current().line_width, 10.0);
        stack.restore();
        assert_eq!(stack.current().line_width, 5.0);
    }

    #[test]
    fn test_restore_empty() {
        let mut stack = GraphicsStateStack::new();
        stack.current_mut().line_width = 5.0;
        stack.restore(); // Should be a no-op
        assert_eq!(stack.current().line_width, 5.0);
    }

    #[test]
    fn test_nested() {
        let mut stack = GraphicsStateStack::new();
        stack.current_mut().line_width = 1.0;
        stack.save();
        stack.current_mut().line_width = 2.0;
        stack.save();
        stack.current_mut().line_width = 3.0;
        assert_eq!(stack.depth(), 2);
        stack.restore();
        assert_eq!(stack.current().line_width, 2.0);
        stack.restore();
        assert_eq!(stack.current().line_width, 1.0);
    }
}
