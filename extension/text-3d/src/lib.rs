use cosmic_text::CacheKey;
use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_scene_core::SceneModelEntity;
use rendiation_texture_core::GPUBufferImage;
use rendiation_texture_gpu_base::create_gpu_texture2d;

mod gles_draw;
mod slug_shader;
pub use gles_draw::use_text3d_gles_renderer;

pub fn register_text3d_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelText3dPayload>(sparse);

  global_database()
    .declare_entity::<Text3dEntity>()
    .declare_component::<Text3dContent>();
}

use facet::Facet;
use serde::*;
#[derive(Debug, Clone, Serialize, Deserialize, Facet)]
pub struct Text3dContentInfo {
  content: String,
  font_size: f32,
  font: Option<u32>,
  weight: Option<u32>,
  color: Vec4<f32>,
}

declare_foreign_key!(SceneModelText3dPayload, SceneModelEntity, Text3dEntity);

declare_entity!(Text3dEntity);
declare_component!(
  Text3dContent,
  Text3dEntity,
  Option<ExternalRefPtr<Text3dContentInfo>>
);

pub struct FontSystem {
  system: cosmic_text::FontSystem,
  swash: cosmic_text::SwashCache,
}

// #[test]
// fn test_font_system() {
//   let mut system = FontSystem::new();
//   // system.build_text_slug_data("Hello, Rust!\n Hello, World! 我是中文");
// }

impl FontSystem {
  pub fn new() -> Self {
    Self {
      system: cosmic_text::FontSystem::new(),
      swash: cosmic_text::SwashCache::new(),
    }
  }
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
  x_min: f32,
  y_min: f32,
  x_max: f32,
  y_max: f32,
}

impl Bounds {
  pub fn size(&self) -> (f32, f32) {
    (self.x_max - self.x_min, self.y_max - self.y_min)
  }
  pub fn empty() -> Self {
    Self {
      x_min: f32::MAX,
      y_min: f32::MAX,
      x_max: -f32::MAX,
      y_max: -f32::MAX,
    }
  }
  pub fn expand(&mut self, x: f32, y: f32) {
    self.x_min = self.x_min.min(x);
    self.y_min = self.y_min.min(y);
    self.x_max = self.x_max.max(x);
    self.y_max = self.y_max.max(y);
  }
}

struct GlyphBands {
  h_bands: Vec<Vec<u32>>,
  v_bands: Vec<Vec<u32>>,
  h_band_count: u32,
  v_band_count: u32,
}

/// Organize curves into horizontal and vertical bands.
fn build_bands(curves: &[f32], bounds: Bounds, band_count: u32) -> GlyphBands {
  let Bounds { x_min, y_min, .. } = bounds;
  let (width, height) = bounds.size();

  let mut h_bands = Vec::new();
  for _ in 0..band_count {
    h_bands.push(Vec::new());
  }

  let mut v_bands = Vec::new();
  for _ in 0..band_count {
    v_bands.push(Vec::new());
  }

  for (ci, [p0x, p0y, p1x, p1y, p2x, p2y]) in curves.as_chunks::<6>().0.iter().enumerate() {
    let ci = ci as u32;
    let cy_min = p0y.min(p1y.min(*p2y));
    let cy_max = p0y.max(p1y.max(*p2y));
    let cx_min = p0x.min(p1x.min(*p2x));
    let cx_max = p0x.max(p1x.max(*p2x));

    if height > 0. {
      let b0 = (cy_min - y_min) / height * band_count as f32;
      let b0 = (b0.floor() as u32).max(0);

      let b1 = (cy_max - y_min) / height * band_count as f32;
      let b1 = (b1.floor() as u32).min(band_count - 1);
      for b in b0..=b1 {
        h_bands[b as usize].push(ci);
      }
    }

    if width > 0. {
      let b0 = (cx_min - x_min) / width * band_count as f32;
      let b0 = (b0.floor() as u32).max(0);

      let b1 = (cx_max - x_min) / width * band_count as f32;
      let b1 = (b1.floor() as u32).min(band_count - 1);

      for b in b0..=b1 {
        v_bands[b as usize].push(ci);
      }
    }
  }

  GlyphBands {
    h_bands,
    v_bands,
    h_band_count: band_count,
    v_band_count: band_count,
  }
}

pub struct SlugGlyph {
  glyph_key: CacheKey,
  curves: Vec<f32>,
  bands: GlyphBands,
  bounds: Bounds,
}

const TEX_WIDTH: usize = 4096;

/// Pack glyph data into GPU textures (RGBA32Float for curves, RGBA32Uint for bands).
fn pack_glyph_data(glyphs: &Vec<SlugGlyph>) -> PackedGlyphData {
  // --- Curve texture (RGBA32Float, width 4096) ---
  // Each curve = 2 texels: (p0x, p0y, p1x, p1y) and (p2x, p2y, 0, 0)
  let mut total_curve_texels = 0;
  for g in glyphs {
    total_curve_texels += g.curves.len() / 6 * 2;
  }

  let curve_tex_height = total_curve_texels.div_ceil(TEX_WIDTH).max(1);
  let mut curve_tex_data = vec![0.0; TEX_WIDTH * curve_tex_height * 4];

  let mut curve_texel_idx = 0;
  let mut glyph_curve_starts = Vec::new();

  for g in glyphs {
    glyph_curve_starts.push(curve_texel_idx);
    for [p0x, p0y, p1x, p1y, p2x, p2y] in g.curves.as_chunks::<6>().0 {
      // Texel 0: (p0x, p0y, p1x, p1y)
      let i0 = curve_texel_idx;
      let x0 = i0 % TEX_WIDTH;
      let y0 = (i0 / TEX_WIDTH) | 0;
      let off0 = (y0 * TEX_WIDTH + x0) * 4;
      curve_tex_data[off0] = *p0x;
      curve_tex_data[off0 + 1] = *p0y;
      curve_tex_data[off0 + 2] = *p1x;
      curve_tex_data[off0 + 3] = *p1y;

      // Texel 1: (p2x, p2y, 0, 0)
      let i1 = curve_texel_idx + 1;
      let x1 = i1 % TEX_WIDTH;
      let y1 = (i1 / TEX_WIDTH) | 0;
      let off1 = (y1 * TEX_WIDTH + x1) * 4;
      curve_tex_data[off1] = *p2x;
      curve_tex_data[off1 + 1] = *p2y;

      curve_texel_idx += 2;
    }
  }

  // --- Band texture (RGBA32Uint, width 4096) ---
  // Per glyph: [hBand headers...] [vBand headers...] [curve index lists...]
  // Each header texel: (curveCount, offsetFromGlyphLoc, 0, 0)
  // Each curve ref texel: (curveTexX, curveTexY, 0, 0)
  let mut total_band_texels = 0;
  for g in glyphs {
    let header_count = g.bands.h_band_count + g.bands.v_band_count;
    // Pad to avoid header wrapping at row boundary
    let padded = TEX_WIDTH - (total_band_texels % TEX_WIDTH);
    if padded < header_count as usize && padded < TEX_WIDTH {
      total_band_texels += padded;
    }
    total_band_texels += header_count as usize;
    for band in &g.bands.h_bands {
      total_band_texels += band.len();
    }
    for band in &g.bands.v_bands {
      total_band_texels += band.len();
    }
  }

  let band_tex_height = total_band_texels.div_ceil(TEX_WIDTH).max(1);
  let mut band_tex_data = vec![0_u32; TEX_WIDTH * band_tex_height * 4];

  let mut band_texel_idx = 0;

  let mut glyph_band_info: Vec<GlyphBandInfo> = Vec::new();

  for (gi, g) in glyphs.iter().enumerate() {
    let h_band_count = g.bands.h_band_count;
    let v_band_count = g.bands.v_band_count;
    let header_count = h_band_count + v_band_count;

    // Ensure headers don't straddle a row boundary
    let cur_x = band_texel_idx % TEX_WIDTH;
    if cur_x + header_count as usize > TEX_WIDTH {
      band_texel_idx = (((band_texel_idx / TEX_WIDTH) | 0) + 1) * TEX_WIDTH;
    }

    let glyph_loc_x = band_texel_idx % TEX_WIDTH;
    let glyph_loc_y = (band_texel_idx / TEX_WIDTH) | 0;
    glyph_band_info.push(GlyphBandInfo {
      glyph_loc_x,
      glyph_loc_y,
    });

    // Sort curves: h-bands by descending max x, v-bands by descending max y
    // const sortedHBands = g.bands.hBands.map(band => ({
    //   curveIndices: [...band.curveIndices].sort((a, b) => {
    //     const ca = g.curves[a], cb = g.curves[b];
    //     return Math.max(cb.p0x, cb.p1x, cb.p2x) - Math.max(ca.p0x, ca.p1x, ca.p2x);
    //   }),
    // }));
    // const sortedVBands = g.bands.vBands.map(band => ({
    //   curveIndices: [...band.curveIndices].sort((a, b) => {
    //     const ca = g.curves[a], cb = g.curves[b];
    //     return Math.max(cb.p0y, cb.p1y, cb.p2y) - Math.max(ca.p0y, ca.p1y, ca.p2y);
    //   }),
    // }));

    //   const allBands = [...sortedHBands, ...sortedVBands];
    let allBands: Vec<_> = g
      .bands
      .h_bands
      .iter()
      .chain(g.bands.v_bands.iter())
      .collect();

    // Calculate offsets: curve lists follow all headers
    let mut curve_list_offset = header_count;
    let mut band_offsets: Vec<u32> = Vec::new();
    for band in &allBands {
      band_offsets.push(curve_list_offset);
      curve_list_offset += band.len() as u32;
    }

    let glyph_start = band_texel_idx;
    let glyph_curve_start = glyph_curve_starts[gi];

    // Write band headers
    for (i, band) in allBands.iter().enumerate() {
      let tl = glyph_start + i;
      let tx = tl % TEX_WIDTH;
      let ty = (tl / TEX_WIDTH) | 0;
      let di = (ty * TEX_WIDTH + tx) * 4;
      band_tex_data[di] = band.len() as u32;
      band_tex_data[di + 1] = band_offsets[i];
    }

    // Write curve index lists (each entry = curve's 2D location in curve texture)
    for (i, band) in allBands.iter().enumerate() {
      let list_start = glyph_start + band_offsets[i] as usize;
      for (j, &ci) in band.iter().enumerate() {
        let curve_texel = glyph_curve_start + ci as usize * 2;
        let c_tex_x = curve_texel % TEX_WIDTH;
        let c_tex_y = (curve_texel / TEX_WIDTH) | 0;

        let tl = list_start + j;
        let tx = tl % TEX_WIDTH;
        let ty = (tl / TEX_WIDTH) | 0;
        let di = (ty * TEX_WIDTH + tx) * 4;
        band_tex_data[di] = c_tex_x as u32;
        band_tex_data[di + 1] = c_tex_y as u32;
      }
    }

    band_texel_idx = glyph_start + curve_list_offset as usize;
  }

  PackedGlyphData {
    curve_tex_data,
    band_tex_data,
    curve_tex_height,
    band_tex_height,
    glyph_band_info,
    glyph_curve_starts,
  }
}

pub struct PackedGlyphData {
  curve_tex_data: Vec<f32>,
  band_tex_data: Vec<u32>,
  curve_tex_height: usize,
  band_tex_height: usize,
  glyph_band_info: Vec<GlyphBandInfo>,
  glyph_curve_starts: Vec<usize>,
}
struct GlyphBandInfo {
  glyph_loc_x: usize,
  glyph_loc_y: usize,
}

fn extract_curves(cmds: &[cosmic_text::Command], curves: &mut Vec<f32>) -> Option<Bounds> {
  let mut current_x = 0.0;
  let mut current_y = 0.0;

  let mut start_x = 0.0;
  let mut start_y = 0.0;

  let start = curves.len();

  for cmd in cmds {
    match cmd {
      cosmic_text::Command::MoveTo(vector) => {
        current_x = vector.x;
        current_y = vector.y;
        start_x = vector.x;
        start_y = vector.y;
      }
      cosmic_text::Command::LineTo(vector) => {
        let mx = (current_x + vector.x) / 2.;
        let my = (current_y + vector.y) / 2.;

        curves.extend([current_x, current_y, mx, my, vector.x, vector.y]);

        current_x = vector.x;
        current_y = vector.y;
      }
      cosmic_text::Command::CurveTo(vector, vector1, vector2) => {
        let cx1 = vector.x;
        let cy1 = vector.y;
        let cx2 = vector1.x;
        let cy2 = vector1.y;
        let ex = vector2.x;
        let ey = vector2.y;

        let m01x = (current_x + cx1) / 2.;
        let m01y = (current_y + cy1) / 2.;
        let m12x = (cx1 + cx2) / 2.;
        let m12y = (cy1 + cy2) / 2.;
        let m23x = (cx2 + ex) / 2.;
        let m23y = (cy2 + ey) / 2.;
        let m012x = (m01x + m12x) / 2.;
        let m012y = (m01y + m12y) / 2.;
        let m123x = (m12x + m23x) / 2.;
        let m123y = (m12y + m23y) / 2.;
        let midx = (m012x + m123x) / 2.;
        let midy = (m012y + m123y) / 2.;

        curves.extend([current_x, current_y, m01x, m01y, midx, midy]);
        curves.extend([midx, midy, m123x, m123y, ex, ey]);

        current_x = ex;
        current_y = ey;
      }
      cosmic_text::Command::QuadTo(vector, vector1) => {
        curves.extend([
          current_x, current_y, vector.x, vector.y, vector1.x, vector1.y,
        ]);
        current_x = vector1.x;
        current_y = vector1.y;
      }
      cosmic_text::Command::Close => {
        let cdx = start_x - current_x;
        let cdy = start_y - current_y;

        if cdx.abs() > 0.01 || cdy.abs() > 0.01 {
          let mx = (current_x + start_x) / 2.;
          let my = (current_y + start_y) / 2.;
          curves.extend([current_x, current_y, mx, my, start_x, start_y]);
        }
        current_x = start_x;
        current_y = start_y;
      }
    }
  }

  if curves.len() - start <= 6 {
    return None;
  }

  let mut bound = Bounds::empty();
  let new_points = curves[start..].as_chunks::<2>().0;
  for [x, y] in new_points {
    bound.expand(*x, *y);
  }

  Some(bound)
}

/// Prepare all glyph data for a text string.
/// Returns texture data and 5-attribute vertex buffers matching the Slug shaders.
fn prepare_text(system: &mut FontSystem, input: &Text3dContentInfo) -> SlugTextPrepared {
  // Text metrics indicate the font size and line height of a buffer
  let metrics = cosmic_text::Metrics::new(14.0, 20.0);

  // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
  let mut buffer = cosmic_text::Buffer::new(&mut system.system, metrics);

  // Set a size for the text buffer, in pixels
  buffer.set_size(&mut system.system, Some(80.0), Some(25.0));

  // Attributes indicate what font to choose
  let attrs = cosmic_text::Attrs::new();

  // Add some text!
  buffer.set_text(
    &mut system.system,
    &input.content,
    &attrs,
    cosmic_text::Shaping::Advanced,
    None,
  );

  // Perform shaping as desired
  buffer.shape_until_scroll(&mut system.system, true);

  let mut used_glyphs = FastHashSet::default();

  struct PositionedGlyph {
    glyph_key: CacheKey,
    /// the bounding's origin relative to glyph space origin?
    relative_x: f32,
    relative_y: f32,
  }

  let mut glyph_buffer: Vec<PositionedGlyph> = Vec::new();

  // Inspect the output runs
  for run in buffer.layout_runs() {
    // println!("{:#?}", run);
    for glyph in run.glyphs.iter() {
      let cache_key = glyph.physical((0., run.line_y), 1.0).cache_key;
      used_glyphs.insert(cache_key);
      glyph_buffer.push(PositionedGlyph {
        glyph_key: cache_key,
        relative_x: glyph.x_offset,
        relative_y: glyph.y_offset,
      });
    }
  }

  let mut glyph_map = FastHashMap::default();

  for cache_key in &used_glyphs {
    if let Some(outline_cmds) = system
      .swash
      .get_outline_commands(&mut system.system, *cache_key)
    {
      let mut curves = Vec::new();
      if let Some(bounds) = extract_curves(&outline_cmds, &mut curves) {
        let bands = build_bands(&curves, bounds, 8);
        glyph_map.insert(
          cache_key,
          SlugGlyph {
            glyph_key: *cache_key,
            curves,
            bands,
            bounds,
          },
        );
      }
    } else {
      log::warn!(
        "unable to get outline commands of glyph {}",
        cache_key.glyph_id
      );
    }
  }

  let scale: f32 = 1.0;

  let slug_glyphs = glyph_map.drain().map(|(_, v)| v).collect::<Vec<_>>();
  let packed = pack_glyph_data(&slug_glyphs);

  // Build per-glyph lookup
  let mut glyph_data_map: FastHashMap<CacheKey, (&SlugGlyph, usize, usize)> =
    FastHashMap::default();

  for (index, slug_glyph) in slug_glyphs.iter().enumerate() {
    let band_info = &packed.glyph_band_info[index];
    glyph_data_map.insert(
      slug_glyph.glyph_key,
      (slug_glyph, band_info.glyph_loc_x, band_info.glyph_loc_y),
    );
  }

  // Build vertex/index data
  // 5 attributes × vec4 = 20 floats = 80 bytes per vertex
  let mut verts = Vec::new();
  let mut idxs = Vec::new();
  let mut cursor_x = 0.;
  let mut quad_idx = 0;

  // for (const { info, position } of glyphBuffer) {
  for positioned_glyph in &glyph_buffer {
    let data = glyph_data_map.get(&positioned_glyph.glyph_key).unwrap();
    //   if (!data) {
    //     cursorX += position.xAdvance;
    //     continue;
    //   }

    let (glyph, glyph_loc_x, glyph_loc_y) = data;
    let Bounds {
      x_min,
      y_min,
      x_max,
      y_max,
    } = glyph.bounds;
    let (w, h) = glyph.bounds.size();

    // Object-space position (Y-up screen pixels)
    let ox = (cursor_x + positioned_glyph.relative_x) * scale;
    let oy = positioned_glyph.relative_y * scale;
    let x0 = ox + x_min * scale; // todo, why?
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

    // Pack tex.z: glyph location in band texture (u16 x, u16 y → bitcast to f32)
    let glyph_loc_packed = f32::from_bits(((*glyph_loc_y as u32) << 16) | *glyph_loc_x as u32);

    // Pack tex.w: band max indices (bandMaxX in bits 0-7, bandMaxY in bits 16-23)
    let band_max_x = glyph.bands.v_band_count - 1;
    let band_max_y = glyph.bands.h_band_count - 1;
    let band_max_packed = f32::from_bits((band_max_y << 16) | band_max_x);

    // Inverse Jacobian: d(em)/d(obj) = 1/scale (uniform scaling)
    let inv_scale = 1. / scale;

    // 4 corners: (objX, objY, normX, normY, emX, emY)
    let corners = [
      [x0, y0, -1., -1., x_min, y_min], // bottom-left
      [x1, y0, 1., -1., x_max, y_min],  // bottom-right
      [x1, y1, 1., 1., x_max, y_max],   // top-right
      [x0, y1, -1., 1., x_min, y_max],  // top-left
    ];

    for [px, py, nx, ny, ex, ey] in corners {
      #[rustfmt::skip]
      verts.extend([
        // pos (location 0): object-space position + normal
        px, py, nx, ny,
        // tex (location 1): em-space coords + packed glyph/band data
        ex, ey, glyph_loc_packed, band_max_packed,
        // jac (location 2): inverse Jacobian (d(em)/d(obj))
        inv_scale, 0., 0., inv_scale,
        // bnd (location 3): band transform (scale + offset)
        band_scale_x, band_scale_y, band_offset_x, band_offset_y,
        // col (location 4): vertex color
        1., 1., 1., 1.,]
      );
    }

    let base = quad_idx * 4;
    idxs.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    // cursorX += position.xAdvance; todo
    quad_idx += 1;
  }

  drop(glyph_data_map);

  return SlugTextPrepared {
    slug_glyphs,
    vertices: verts,
    indices: idxs,
    packed,
    // totalAdvance: cursorX,
  };
}

pub struct SlugTextPrepared {
  pub slug_glyphs: Vec<SlugGlyph>,
  pub vertices: Vec<f32>,
  pub indices: Vec<u32>,
  pub packed: PackedGlyphData,
  // totalAdvance: u32,
}
