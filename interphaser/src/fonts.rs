use std::collections::HashMap;

use glyph_brush::{ab_glyph, FontId};

pub struct FontManager {
  fonts_by_name: HashMap<String, (ab_glyph::FontArc, FontId)>,
  fonts: Vec<ab_glyph::FontArc>,
}

impl FontManager {
  pub fn new_with_fallback_system_font(fall_back_font_name: &str) -> Self {
    let property = font_loader::system_fonts::FontPropertyBuilder::new()
      .family("Arial")
      .build();

    let (font, _) = font_loader::system_fonts::get(&property).unwrap();
    let default_font = ab_glyph::FontArc::try_from_vec(font).unwrap();

    let mut fonts = Self {
      fonts: Vec::new(),
      fonts_by_name: HashMap::new(),
    };

    fonts.add_font(fall_back_font_name, default_font);
    fonts
  }

  pub fn add_font(&mut self, name: &str, font: ab_glyph::FontArc) -> FontId {
    self
      .fonts_by_name
      .entry(name.to_owned())
      .or_insert_with(|| {
        let index = self.fonts.len();
        self.fonts.push(font.clone());
        (font, FontId(index))
      })
      .1
  }

  pub fn get_font(&self, id: FontId) -> &ab_glyph::FontArc {
    self.fonts.get(id.0).unwrap()
  }

  pub fn get_font_id_or_fallback(&self, name: &str) -> FontId {
    if let Some(font) = self.fonts_by_name.get(name) {
      font.1
    } else {
      FontId(0)
    }
  }

  pub fn get_fonts(&self) -> &Vec<ab_glyph::FontArc> {
    &self.fonts
  }
}
