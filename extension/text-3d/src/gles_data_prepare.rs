use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

/// Prepare all glyph data for a text string.
/// Returns texture data and 5-attribute vertex buffers matching the Slug shaders.
///
/// if something wrong or the content is empty, return None
pub fn prepare_gles_text(input: &SlugBuffer, font_sys: &FontSystem) -> Option<SlugTextPrepared> {
  let SlugBuffer {
    positions: glyph_buffer,
    unique_glyphs,
    scale,
  } = input;

  let (packed, glyph_data_map) = pack_glyph_data(unique_glyphs, font_sys);

  // Build vertex/index data
  // 5 attributes × vec4 = 20 floats = 80 bytes per vertex
  let mut verts = Vec::new();
  let mut idxs = Vec::new();
  let mut quad_idx = 0;

  for positioned_glyph in glyph_buffer {
    let data = glyph_data_map.get(&positioned_glyph.glyph_key);
    if data.is_none() {
      continue;
    }
    let data = data.unwrap();

    let glyph = font_sys.get_computed_slug_glyph(&positioned_glyph.glyph_key);
    if glyph.is_none() {
      continue;
    }
    let glyph = glyph.unwrap();

    let GlyphBandInfo {
      glyph_loc_x,
      glyph_loc_y,
    } = data;
    let x_min = glyph.bounds.min.x;
    let y_min = glyph.bounds.min.y;
    let x_max = glyph.bounds.max.x;
    let y_max = glyph.bounds.max.y;
    let (w, h) = glyph.bounds.size();

    // Object-space position (Y-up screen pixels)
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
    quad_idx += 1;
  }

  drop(glyph_data_map);

  if verts.is_empty() || idxs.is_empty() {
    return None;
  }

  Some(SlugTextPrepared {
    vertices: verts,
    indices: idxs,
    packed,
  })
}

pub struct SlugTextPrepared {
  pub vertices: Vec<f32>,
  pub indices: Vec<u32>,
  pub packed: PackedGlyphData,
}

pub const TEX_WIDTH: usize = 4096;

pub struct PackedGlyphData {
  pub curve_tex_data: Vec<f32>,
  pub band_tex_data: Vec<u32>,
  pub curve_tex_height: usize,
  pub band_tex_height: usize,
}

pub struct GlyphBandInfo {
  pub glyph_loc_x: usize,
  pub glyph_loc_y: usize,
}

/// Pack glyph data into GPU textures (RGBA32Float for curves, RGBA32Uint for bands).
fn pack_glyph_data(
  glyphs: &FastHashSet<CacheKey>,
  font_sys: &FontSystem,
) -> (PackedGlyphData, FastHashMap<CacheKey, GlyphBandInfo>) {
  let filter = |g| font_sys.get_computed_slug_glyph(g);

  // --- Curve texture (RGBA32Float, width 4096) ---
  // Each curve = 2 texels: (p0x, p0y, p1x, p1y) and (p2x, p2y, 0, 0)
  let mut total_curve_texels = 0;
  for g in glyphs.iter().filter_map(filter) {
    total_curve_texels += g.curves.len() / 6 * 2;
  }

  let curve_tex_height = total_curve_texels.div_ceil(TEX_WIDTH).max(1);
  let mut curve_tex_data = vec![0.0; TEX_WIDTH * curve_tex_height * 4];

  let mut curve_texel_idx = 0;
  let mut glyph_curve_starts = Vec::new();

  for g in glyphs.iter().filter_map(filter) {
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
  for g in glyphs.iter().filter_map(filter) {
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

  // Build per-glyph lookup
  let mut glyph_data_map: FastHashMap<CacheKey, GlyphBandInfo> = FastHashMap::default();

  for (gi, g) in glyphs.iter().filter_map(filter).enumerate() {
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

    glyph_data_map.insert(
      g.glyph_key,
      GlyphBandInfo {
        glyph_loc_x,
        glyph_loc_y,
      },
    );

    let all_bands: Vec<_> = g
      .bands
      .h_bands
      .iter()
      .chain(g.bands.v_bands.iter())
      .collect();

    // Calculate offsets: curve lists follow all headers
    let mut curve_list_offset = header_count;
    let mut band_offsets: Vec<u32> = Vec::new();
    for band in &all_bands {
      band_offsets.push(curve_list_offset);
      curve_list_offset += band.len() as u32;
    }

    let glyph_start = band_texel_idx;
    let glyph_curve_start = glyph_curve_starts[gi];

    // Write band headers
    for (i, band) in all_bands.iter().enumerate() {
      let tl = glyph_start + i;
      let tx = tl % TEX_WIDTH;
      let ty = (tl / TEX_WIDTH) | 0;
      let di = (ty * TEX_WIDTH + tx) * 4;
      band_tex_data[di] = band.len() as u32;
      band_tex_data[di + 1] = band_offsets[i];
    }

    // Write curve index lists (each entry = curve's 2D location in curve texture)
    for (i, band) in all_bands.iter().enumerate() {
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

  let packed = PackedGlyphData {
    curve_tex_data,
    band_tex_data,
    curve_tex_height,
    band_tex_height,
  };
  (packed, glyph_data_map)
}

fn create_gpu_texture2d_impl(cx: &GPU, texture: &GPUBufferImage) -> GPU2DTextureView {
  let texture = GPUBufferImageForeignImpl { inner: texture };

  let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap, cx.info().downgrade_info.flags);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);

  gpu_texture.create_default_view().try_into().unwrap()
}

fn create_gpu_texture2d_u32(
  cx: &GPU,
  texture: &GPUBufferImage,
) -> GPUTypedTextureView<TextureDimension2, u32> {
  let texture = GPUBufferImageForeignImpl { inner: texture };

  let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap, cx.info().downgrade_info.flags);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);

  gpu_texture.create_default_view().try_into().unwrap()
}

impl SlugTextPrepared {
  pub fn create_gpu(&self, gpu: &GPU) -> SlugTextGPUData {
    let indices = create_gpu_buffer(
      bytemuck::cast_slice(&self.indices),
      BufferUsages::INDEX,
      &gpu.device,
    );
    let indices = indices.create_default_view();
    let vertices = create_gpu_buffer(
      bytemuck::cast_slice(&self.vertices),
      BufferUsages::VERTEX,
      &gpu.device,
    );
    let vertices = vertices.create_default_view();

    let curve_tex_data = create_gpu_texture2d_impl(
      gpu,
      &GPUBufferImage {
        data: bytemuck::cast_slice(&self.packed.curve_tex_data).to_vec(),
        format: TextureFormat::Rgba32Float,
        size: Size::from_u32_pair_min_one((TEX_WIDTH as u32, self.packed.curve_tex_height as u32)),
      },
    );

    let band_tex_data = create_gpu_texture2d_u32(
      gpu,
      &GPUBufferImage {
        data: bytemuck::cast_slice(&self.packed.band_tex_data).to_vec(),
        format: TextureFormat::Rgba32Uint,
        size: Size::from_u32_pair_min_one((TEX_WIDTH as u32, self.packed.band_tex_height as u32)),
      },
    )
    .texture
    .try_into()
    .unwrap();

    SlugTextGPUData {
      indices,
      vertices,
      curve_tex_data,
      band_tex_data,
    }
  }
}
