use super::GlyphID;
use rendiation_algebra::Vec2;
use rendiation_texture::Texture2DBuffer;

pub trait GlyphRaster {
  fn raster(&mut self, glyph_id: GlyphID, info: GlyphRasterInfo) -> Texture2DBuffer<u8>;
}

#[derive(Clone, Copy, PartialEq)]
pub struct GlyphRasterInfo {
  // position in pixel
  position: Vec2<f32>,
  // pixel-height of text.
  scale: Vec2<f32>,
}

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

pub struct AbGlyphRaster {}

impl GlyphRaster for AbGlyphRaster {
  fn raster(&mut self, glyph_id: GlyphID, info: GlyphRasterInfo) -> Texture2DBuffer<u8> {
    todo!()
  }
}
