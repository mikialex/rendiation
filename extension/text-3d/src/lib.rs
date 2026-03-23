use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_scene_core::SceneModelEntity;
use rendiation_texture_core::GPUBufferImage;
use rendiation_texture_gpu_base::create_gpu_texture2d;
use rendiation_webgpu::*;

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

#[test]
fn test_font_system() {
  let mut system = FontSystem::new();
  // system.build_text_slug_data("Hello, Rust!\n Hello, World! 我是中文");
}

impl FontSystem {
  pub fn new() -> Self {
    Self {
      system: cosmic_text::FontSystem::new(),
      swash: cosmic_text::SwashCache::new(),
    }
  }
}

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

struct SlugGlyph {
  glyph_id: u32,
  curves: Vec<f32>,
  bands: GlyphBands,
  bounds: Bounds,
}

const TEX_WIDTH: usize = 4096;

/// Pack glyph data into GPU textures (RGBA32Float for curves, RGBA32Uint for bands).
fn pack_glyph_data(glyphs: Vec<SlugGlyph>) -> PackedGlyphData {
  // --- Curve texture (RGBA32Float, width 4096) ---
  // Each curve = 2 texels: (p0x, p0y, p1x, p1y) and (p2x, p2y, 0, 0)
  let mut totalCurveTexels = 0;
  for g in &glyphs {
    totalCurveTexels += g.curves.len() / 6 * 2;
  }

  let curveTexHeight = totalCurveTexels.div_ceil(TEX_WIDTH).max(1);
  let mut curveTexData = vec![0.0; TEX_WIDTH * curveTexHeight * 4];

  let mut curveTexelIdx = 0;
  let mut glyphCurveStarts = Vec::new();

  for g in &glyphs {
    glyphCurveStarts.push(curveTexelIdx);
    for [p0x, p0y, p1x, p1y, p2x, p2y] in g.curves.as_chunks::<6>().0 {
      // Texel 0: (p0x, p0y, p1x, p1y)
      let i0 = curveTexelIdx;
      let x0 = i0 % TEX_WIDTH;
      let y0 = (i0 / TEX_WIDTH) | 0;
      let off0 = (y0 * TEX_WIDTH + x0) * 4;
      curveTexData[off0] = *p0x;
      curveTexData[off0 + 1] = *p0y;
      curveTexData[off0 + 2] = *p1x;
      curveTexData[off0 + 3] = *p1y;

      // Texel 1: (p2x, p2y, 0, 0)
      let i1 = curveTexelIdx + 1;
      let x1 = i1 % TEX_WIDTH;
      let y1 = (i1 / TEX_WIDTH) | 0;
      let off1 = (y1 * TEX_WIDTH + x1) * 4;
      curveTexData[off1] = *p2x;
      curveTexData[off1 + 1] = *p2y;

      curveTexelIdx += 2;
    }
  }

  // --- Band texture (RGBA32Uint, width 4096) ---
  // Per glyph: [hBand headers...] [vBand headers...] [curve index lists...]
  // Each header texel: (curveCount, offsetFromGlyphLoc, 0, 0)
  // Each curve ref texel: (curveTexX, curveTexY, 0, 0)
  let mut totalBandTexels = 0;
  for g in &glyphs {
    let headerCount = g.bands.h_band_count + g.bands.v_band_count;
    // Pad to avoid header wrapping at row boundary
    let padded = TEX_WIDTH - (totalBandTexels % TEX_WIDTH);
    if padded < headerCount as usize && padded < TEX_WIDTH {
      totalBandTexels += padded;
    }
    totalBandTexels += headerCount as usize;
    for band in &g.bands.h_bands {
      totalBandTexels += band.len();
    }
    for band in &g.bands.v_bands {
      totalBandTexels += band.len();
    }
  }

  let bandTexHeight = totalBandTexels.div_ceil(TEX_WIDTH).max(1);
  let mut bandTexData = vec![0_u32; TEX_WIDTH * bandTexHeight * 4];

  let mut bandTexelIdx = 0;

  let mut glyphBandInfo: Vec<GlyphBandInfo> = Vec::new();

  for (gi, g) in glyphs.iter().enumerate() {
    let hBandCount = g.bands.h_band_count;
    let vBandCount = g.bands.v_band_count;
    let headerCount = hBandCount + vBandCount;

    // Ensure headers don't straddle a row boundary
    let curX = bandTexelIdx % TEX_WIDTH;
    if (curX + headerCount as usize > TEX_WIDTH) {
      bandTexelIdx = (((bandTexelIdx / TEX_WIDTH) | 0) + 1) * TEX_WIDTH;
    }

    let glyphLocX = bandTexelIdx % TEX_WIDTH;
    let glyphLocY = (bandTexelIdx / TEX_WIDTH) | 0;
    glyphBandInfo.push(GlyphBandInfo {
      glyphLocX,
      glyphLocY,
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
    let mut curveListOffset = headerCount;
    let mut bandOffsets: Vec<u32> = Vec::new();
    for band in &allBands {
      bandOffsets.push(curveListOffset);
      curveListOffset += band.len() as u32;
    }

    let glyphStart = bandTexelIdx;
    let glyphCurveStart = glyphCurveStarts[gi];

    // Write band headers
    for (i, band) in allBands.iter().enumerate() {
      let tl = glyphStart + i;
      let tx = tl % TEX_WIDTH;
      let ty = (tl / TEX_WIDTH) | 0;
      let di = (ty * TEX_WIDTH + tx) * 4;
      bandTexData[di] = band.len() as u32;
      bandTexData[di + 1] = bandOffsets[i];
    }

    // Write curve index lists (each entry = curve's 2D location in curve texture)
    for (i, band) in allBands.iter().enumerate() {
      let listStart = glyphStart + bandOffsets[i] as usize;
      for (j, &ci) in band.iter().enumerate() {
        let curveTexel = glyphCurveStart + ci as usize * 2;
        let cTexX = curveTexel % TEX_WIDTH;
        let cTexY = (curveTexel / TEX_WIDTH) | 0;

        let tl = listStart + j;
        let tx = tl % TEX_WIDTH;
        let ty = (tl / TEX_WIDTH) | 0;
        let di = (ty * TEX_WIDTH + tx) * 4;
        bandTexData[di] = cTexX as u32;
        bandTexData[di + 1] = cTexY as u32;
      }
    }

    bandTexelIdx = glyphStart + curveListOffset as usize;
  }

  PackedGlyphData {
    curveTexData,
    bandTexData,
    curveTexHeight,
    bandTexHeight,
    glyphBandInfo,
    glyphCurveStarts,
  }
}

struct PackedGlyphData {
  curveTexData: Vec<f32>,
  bandTexData: Vec<u32>,
  curveTexHeight: usize,
  bandTexHeight: usize,
  glyphBandInfo: Vec<GlyphBandInfo>,
  glyphCurveStarts: Vec<usize>,
}
struct GlyphBandInfo {
  glyphLocX: usize,
  glyphLocY: usize,
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

  let mut curves = Vec::new();
  // let mut bounds = Vec::new();

  // Inspect the output runs
  for run in buffer.layout_runs() {
    println!("{:#?}", run);
    for glyph in run.glyphs.iter() {
      used_glyphs.insert(glyph.glyph_id);

      if let Some(outline_cmds) = system.swash.get_outline_commands(
        &mut system.system,
        glyph.physical((0., run.line_y), 1.0).cache_key,
      ) {
        extract_curves(&outline_cmds, &mut curves);
        // self.swash.
        // let bounds = self.swash.get_image(font_system, cache_key)
      } else {
        log::warn!("unable to get outline commands of glyph {}", glyph.glyph_id);
      }
    }
  }

  // const buf = new UnicodeBuffer();
  // buf.addStr(text);
  // const glyphBuffer = shape(font, buf);
  // const scale = font.scaleForSize(fontSize);
  let scale: f32 = todo!();

  // // Process unique glyphs
  // const glyphMap = new Map<number, SlugGlyph>();
  // for (const { info } of glyphBuffer) {
  //   if (glyphMap.has(info.glyphId)) continue;
  //   const result = extractCurves(font, info.glyphId);
  //   if (!result) continue;
  //   const bands = buildBands(result.curves, result.bounds);
  //   glyphMap.set(info.glyphId, {
  //     glyphId: info.glyphId,
  //     curves: result.curves,
  //     bands,
  //     bounds: result.bounds,
  //   });
  // }

  // const slugGlyphs = [...glyphMap.values()];
  let slugGlyphs = todo!();
  let packed = pack_glyph_data(slugGlyphs);

  // Build per-glyph lookup
  let glyphDataMap: FastHashMap<u32, (&SlugGlyph, usize, usize)> = FastHashMap::default();

  for (index, slug_glyph) in slugGlyphs.iter().enumerate() {
    let band_info = packed.glyphBandInfo[index];
    glyphDataMap.insert(
      slug_glyph.glyph_id,
      (slug_glyph, band_info.glyphLocX, band_info.glyphLocY),
    );
  }

  // Build vertex/index data
  // 5 attributes × vec4 = 20 floats = 80 bytes per vertex
  let verts = Vec::new();
  let idxs = Vec::new();
  let cursorX = 0.;
  let quadIdx = 0;

  struct PositionedGlyph {
    glyph_id: u32,
    /// the bounding's origin relative to glyph space origin?
    relative_x: f32,
    relative_y: f32,
  }

  let glyphBuffer: Vec<PositionedGlyph> = todo!();

  // for (const { info, position } of glyphBuffer) {
  for positioned_glyph in &glyphBuffer {
    let data = glyphDataMap.get(&positioned_glyph.glyph_id).unwrap();
    //   if (!data) {
    //     cursorX += position.xAdvance;
    //     continue;
    //   }

    let (glyph, glyphLocX, glyphLocY) = data;
    let Bounds {
      x_min,
      y_min,
      x_max,
      y_max,
    } = glyph.bounds;
    let (w, h) = glyph.bounds.size();

    // Object-space position (Y-up screen pixels)
    let ox = (cursorX + positioned_glyph.relative_x) * scale;
    let oy = positioned_glyph.relative_y * scale;
    let x0 = ox + x_min * scale; // todo, why?
    let y0 = oy + y_min * scale;
    let x1 = ox + x_max * scale;
    let y1 = oy + y_max * scale;

    // Band transform: maps em-space to band indices
    let bandScaleX = if w > 0. {
      glyph.bands.v_band_count as f32 / w
    } else {
      0.0
    };
    let bandScaleY = if h > 0. {
      glyph.bands.h_band_count as f32 / h
    } else {
      0.0
    };
    let bandOffsetX = -x_min * bandScaleX;
    let bandOffsetY = -y_min * bandScaleY;

    // Pack tex.z: glyph location in band texture (u16 x, u16 y → bitcast to f32)
    let glyphLocPacked = f32::from_bits(((*glyphLocY as u32) << 16) | *glyphLocX as u32);

    // Pack tex.w: band max indices (bandMaxX in bits 0-7, bandMaxY in bits 16-23)
    let bandMaxX = glyph.bands.v_band_count - 1;
    let bandMaxY = glyph.bands.h_band_count - 1;
    let bandMaxPacked = f32::from_bits((bandMaxY << 16) | bandMaxX);

    // Inverse Jacobian: d(em)/d(obj) = 1/scale (uniform scaling)
    let invScale = 1. / scale;

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
        ex, ey, glyphLocPacked, bandMaxPacked,
        // jac (location 2): inverse Jacobian (d(em)/d(obj))
        invScale, 0., 0., invScale,
        // bnd (location 3): band transform (scale + offset)
        bandScaleX, bandScaleY, bandOffsetX, bandOffsetY,
        // col (location 4): vertex color
        1., 1., 1., 1.,]
      );
    }

    let base = quadIdx * 4;
    idxs.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    // cursorX += position.xAdvance; todo
    quadIdx += 1;
  }

  return SlugTextPrepared {
    slugGlyphs,
    vertices: verts,
    indices: idxs,
    packed,
    // totalAdvance: cursorX,
  };
}

pub struct SlugTextPrepared {
  pub slugGlyphs: Vec<SlugGlyph>,
  pub vertices: Vec<f32>,
  pub indices: Vec<u32>,
  pub packed: PackedGlyphData,
  // totalAdvance: u32,
}
