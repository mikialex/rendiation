use crate::*;

pub trait Font: Any {
  fn raster(&self, glyph_id: GlyphID, info: GlyphRasterInfo) -> Option<Texture2DBuffer<u8>>;
  fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphID(pub(crate) char, pub(crate) FontId);

#[derive(Default)]
pub struct FontManager {
  fonts_by_name: HashMap<String, (Rc<dyn Font>, FontId)>,
  fonts: Vec<Rc<dyn Font>>,
}

impl FontManager {
  pub fn font_count(&self) -> usize {
    self.fonts.len()
  }

  pub fn add_font(&mut self, name: &str, font: impl Font) -> FontId {
    self
      .fonts_by_name
      .entry(name.to_owned())
      .or_insert_with(|| {
        let index = self.fonts.len();
        let font = Rc::new(font);
        self.fonts.push(font.clone());
        (font, FontId(index))
      })
      .1
  }

  pub fn get_font(&self, id: FontId) -> Option<&dyn Font> {
    self.fonts.get(id.0).map(|f| f.as_ref() as &dyn Font)
  }

  pub fn get_fonts(&self) -> Vec<&dyn Font> {
    self.fonts.iter().map(|f| f.as_ref() as &dyn Font).collect()
  }

  pub fn get_font_id_or_fallback(&self, name: &str) -> FontId {
    if let Some(font) = self.fonts_by_name.get(name) {
      font.1
    } else {
      FontId(0)
    }
  }

  pub(crate) fn raster(
    &self,
    glyph_id: GlyphID,
    info: GlyphRasterInfo,
  ) -> Option<Texture2DBuffer<u8>> {
    let font = self.get_font(glyph_id.1)?;
    font.raster(glyph_id, info)
  }
}
