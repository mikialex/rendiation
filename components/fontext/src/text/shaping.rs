use crate::*;

pub struct ShapeRun<'a> {
  /// content to shaping, should be in single line and has same text properties
  pub content: &'a str,
  pub font: FontId,
}

pub struct ShapedGlyph {
  pub font_id: u32,
  pub glyph_id: u32,
  /// An index to the start of the grapheme cluster in the original string.
  ///
  /// [Read more on clusters](https://harfbuzz.github.io/clusters.html).
  pub cluster: u32,
  pub position: GlyphRelativePosition,
}

pub struct ShapedGlyphs {
  pub glyphs: Vec<ShapedGlyph>,
}

/// Holds the positions of the glyph in both horizontal and vertical directions.
///
/// All positions are relative to the current point.
#[derive(Clone, Copy, Default, Debug)]
pub struct GlyphRelativePosition {
  /// How much the line advances after drawing this glyph when setting text in
  /// horizontal direction.
  pub x_advance: i32,
  /// How much the line advances after drawing this glyph when setting text in
  /// vertical direction.
  pub y_advance: i32,
  /// How much the glyph moves on the X-axis before drawing it, this should
  /// not affect how much the line advances.
  pub x_offset: i32,
  /// How much the glyph moves on the Y-axis before drawing it, this should
  /// not affect how much the line advances.
  pub y_offset: i32,
}

pub trait Shaper {
  fn shaping(&self, run: ShapeRun) -> ShapedGlyphs;
}
