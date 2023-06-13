use crate::*;

#[derive(Clone, Copy, PartialEq)]
pub struct GlyphRasterInfo {
  // position in pixel
  pub position: Vec2<f32>,
  // pixel-height of text.
  pub scale: f32,
}

impl GlyphRasterInfo {
  pub fn normalize(&self, tolerance: &GlyphRasterTolerance) -> NormalizedGlyphRasterInfo {
    let scale = self.scale;
    let offset = normalized_offset_from_position(self.position);

    fn normalized_offset_from_position(position: Vec2<f32>) -> Vec2<f32> {
      let mut offset = Vec2::new(position.x.fract(), position.y.fract());
      if offset.x > 0.5 {
        offset.x -= 1.0;
      } else if offset.x < -0.5 {
        offset.x += 1.0;
      }
      if offset.y > 0.5 {
        offset.y -= 1.0;
      } else if offset.y < -0.5 {
        offset.y += 1.0;
      }
      offset
    }

    NormalizedGlyphRasterInfo {
      scale_over_tolerance: (scale / tolerance.scale + 0.5) as u32,
      // convert [-0.5, 0.5] -> [0, 1] then divide
      offset_over_tolerance: (
        ((offset.x + 0.5) / tolerance.position + 0.5) as u16,
        ((offset.y + 0.5) / tolerance.position + 0.5) as u16,
      ),
    }
  }
}

pub struct GlyphRasterTolerance {
  pub scale: f32,
  pub position: f32,
}

impl Default for GlyphRasterTolerance {
  fn default() -> Self {
    Self {
      scale: 0.1,
      position: 0.1,
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedGlyphRasterInfo {
  scale_over_tolerance: u32,
  offset_over_tolerance: (u16, u16),
}

#[allow(clippy::derived_hash_with_manual_eq)]
impl core::hash::Hash for GlyphRasterInfo {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    unsafe {
      let value: &Vec2<u32> = std::mem::transmute(&self.position);
      value.hash(state);

      let value: &Vec2<u32> = std::mem::transmute(&self.scale);
      value.hash(state);
    }
  }
}

impl Eq for GlyphRasterInfo {}
