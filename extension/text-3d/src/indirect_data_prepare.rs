use crate::*;

pub(crate) struct SlugIndirectGPUResource {
  pub curves: Vec<CurveData>,
  pub bands: Vec<u32>,
  pub vertices: Vec<TextGlyphQuad>,
}

pub(crate) fn prepare_indirect_text(
  input: &SlugBuffer,
  font_sys: &FontSystem,
) -> SlugIndirectGPUResource {
  let filter = |g| font_sys.get_computed_slug_glyph(g);

  let glyphs = &input.unique_glyphs;

  // Build per-glyph lookup
  let mut glyph_data_map: FastHashMap<CacheKey, u32> = FastHashMap::default();
  let mut curves = Vec::new();

  // Per glyph: [hBand headers...] [vBand headers...] [curve index lists...]
  // Each header: (curveCount, offsetFromGlyphLoc, 0, 0)
  // Each curve ref: curveIndex
  let mut band_data = Vec::new();
  for g in glyphs.iter().filter_map(filter) {
    glyph_data_map.insert(g.glyph_key, band_data.len() as u32);

    let curve_offset = curves.len() as u32;
    for [p0x, p0y, p1x, p1y, p2x, p2y] in g.curves.as_chunks::<6>().0 {
      curves.push(CurveData {
        p1: (*p0x, *p0y).into(),
        p2: (*p1x, *p1y).into(),
        p3: (*p2x, *p2y).into(),
        ..Default::default()
      });
    }

    band_data.push(curve_offset);
    band_data.push(g.bands.h_band_count);
    band_data.push(g.bands.v_band_count);

    let mut offset = 0;
    for h_band in &g.bands.h_bands {
      band_data.push(offset as u32);
      band_data.push(h_band.len() as u32);
      offset += h_band.len();
    }
    for v_band in &g.bands.v_bands {
      band_data.push(offset as u32);
      band_data.push(v_band.len() as u32);
      offset += v_band.len();
    }

    band_data.extend(g.bands.h_bands.iter().flat_map(|v| v));
    band_data.extend(g.bands.v_bands.iter().flat_map(|v| v));
  }

  let mut vertices = Vec::new();
  for positioned_glyph in &input.positions {
    let data = glyph_data_map.get(&positioned_glyph.glyph_key);
    if data.is_none() {
      continue;
    }
    let index = data.unwrap();
    let glyph = font_sys.get_computed_slug_glyph(&positioned_glyph.glyph_key);
    if glyph.is_none() {
      continue;
    }
    let glyph = glyph.unwrap();

    let x_min = glyph.bounds.min.x;
    let y_min = glyph.bounds.min.y;
    let x_max = glyph.bounds.max.x;
    let y_max = glyph.bounds.max.y;
    let (w, h) = glyph.bounds.size();

    let scale = input.scale;
    let ox = positioned_glyph.relative_x * scale;
    let oy = positioned_glyph.relative_y * scale;
    let x0 = ox + x_min * scale;
    let y0 = oy + y_min * scale;
    let x1 = ox + x_max * scale;
    let y1 = oy + y_max * scale;

    // Band transform: maps em-space to band indices
    let band_scale_x = if w > 0. {
      glyph.bands.v_band_count as f32 / w
    } else {
      0.0
    };
    let band_scale_y = if h > 0. {
      glyph.bands.h_band_count as f32 / h
    } else {
      0.0
    };
    let band_offset_x = -x_min * band_scale_x;
    let band_offset_y = -y_min * band_scale_y;

    let band_transform = (band_scale_x, band_scale_y, band_offset_x, band_offset_y).into();

    let band_max_x = glyph.bands.v_band_count - 1;
    let band_max_y = glyph.bands.h_band_count - 1;

    vertices.push(TextGlyphQuad {
      obj_space_min: (x0, y0).into(),
      obj_space_size: (x1 - x0, y1 - y0).into(),

      em_space_min: (x_min, y_min).into(),
      em_space_size: (x_max - x_min, y_max - y_min).into(),

      band_offset: *index,
      band_max: (band_max_x, band_max_y).into(),

      band_transform,
      ..Default::default()
    });
  }

  SlugIndirectGPUResource {
    curves,
    bands: band_data,
    vertices,
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct CurveData {
  pub p1: Vec2<f32>,
  pub p2: Vec2<f32>,
  pub p3: Vec2<f32>,
}
impl CurveData {
  pub fn u32_size() -> u32 {
    std::mem::size_of::<Self>() as u32 / 4
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct TextGlyphQuad {
  pub obj_space_min: Vec2<f32>,
  pub obj_space_size: Vec2<f32>,

  pub em_space_min: Vec2<f32>,
  pub em_space_size: Vec2<f32>,

  pub band_offset: u32,
  pub band_max: Vec2<u32>,

  pub band_transform: Vec4<f32>,
}
impl TextGlyphQuad {
  pub fn u32_size() -> u32 {
    std::mem::size_of::<Self>() as u32 / 4
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct GlyphMetadata {
  pub curve_start: u32,
  pub curve_count: u32,
  pub band_start: u32,
  pub band_h_count: u32,
  pub band_v_count: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct TextMeta {
  pub text_curves_range: Vec2<u32>,
  pub text_band_range: Vec2<u32>,
  pub text_vertices_range: Vec2<u32>,
}
