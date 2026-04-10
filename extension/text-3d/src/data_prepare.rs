use rendiation_geometry::Box2;
use rendiation_scene_core::GlobalSceneModelWorldMatrix;

use crate::*;

pub struct Text3dSlugBuffer(pub Arc<RwLock<FontSystem>>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for Text3dSlugBuffer {
  type Result =
    impl DualQueryLike<Key = RawEntityHandle, Value = ExternalRefPtr<SlugBuffer>> + 'static;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let font_system = self.0.clone();
    cx.use_dual_query::<Text3dContent>()
      .dual_query_filter_map(|v| v)
      .use_dual_query_execute_map(cx, move || {
        let mut font_system = font_system.make_write_holder();
        move |_, info| {
          ExternalRefPtr::new(create_slug_buffer_from_text3d_content(
            &mut font_system,
            &info,
          ))
        }
      })
  }
}

pub struct Text3dSceneModelWorldBounding(pub Arc<RwLock<FontSystem>>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for Text3dSceneModelWorldBounding {
  type Result = impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>> + 'static;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let font_system = self.0.clone();
    let local_boxes = cx
      .use_shared_dual_query(Text3dSlugBuffer(self.0.clone()))
      .use_dual_query_execute_map(cx, move || {
        let font_system = font_system.make_read_holder();
        move |_, slug_buffer| slug_buffer.compute_local_bounding(&font_system)
      });

    let relation = cx.use_db_rev_ref_tri_view::<SceneModelText3dPayload>();
    let sm_local_bounding = local_boxes.fanout(relation, cx);

    let scene_model_world_mat = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);

    // todo, materialize
    scene_model_world_mat
      .dual_query_intersect(sm_local_bounding)
      .dual_query_map(|(mat, local)| local.into_f64().apply_matrix_into(mat))
  }
}

#[derive(Clone, Copy, Debug)]
pub struct PositionedGlyph {
  pub glyph_key: CacheKey,
  /// relative to glyph space origin
  pub relative_x: f32,
  pub relative_y: f32,
}

pub struct SlugBuffer {
  pub positions: Vec<PositionedGlyph>,
  pub unique_glyphs: FastHashSet<CacheKey>,
  // pub glyphs: Vec<SlugGlyph>,
  pub scale: f32,
}

impl SlugBuffer {
  pub fn compute_local_bounding(&self, font_sys: &FontSystem) -> Box3 {
    let mut bbox = Box3::empty();
    let scale = self.scale;

    for pos in &self.positions {
      if let Some(glyph) = font_sys.get_computed_slug_glyph(&pos.glyph_key) {
        let x_min = glyph.bounds.min.x;
        let y_min = glyph.bounds.min.y;
        let x_max = glyph.bounds.max.x;
        let y_max = glyph.bounds.max.y;

        let ox = (pos.relative_x) * scale;
        let oy = pos.relative_y * scale;
        let x0 = ox + x_min * scale;
        let y0 = oy + y_min * scale;
        let x1 = ox + x_max * scale;
        let y1 = oy + y_max * scale;
        bbox.expand_by_point(Vec3::new(x0, y0, 0.));
        bbox.expand_by_point(Vec3::new(x1, y1, 0.));
      }
    }

    bbox
  }
}

impl std::fmt::Debug for SlugBuffer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SlugBuffer").finish()
  }
}

pub fn create_slug_buffer_from_text3d_content(
  system: &mut FontSystem,
  input: &Text3dContentInfo,
) -> SlugBuffer {
  // Text metrics indicate the font size and line height of a buffer
  let metrics = cosmic_text::Metrics::new(input.font_size, input.font_size * input.line_height);

  // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
  let mut buffer = cosmic_text::Buffer::new(&mut system.system, metrics);

  // Set a size for the text buffer, in pixels
  buffer.set_size(&mut system.system, input.width, input.height);

  let font = input
    .font
    .as_ref()
    .map(|f| cosmic_text::Family::Name(f))
    .unwrap_or(cosmic_text::Family::SansSerif);

  // todo, check if font presented in font system

  // Attributes indicate what font to choose
  let attrs = cosmic_text::Attrs::new()
    .family(font)
    .style(if input.italic {
      cosmic_text::Style::Italic
    } else {
      cosmic_text::Style::Normal
    })
    .weight(cosmic_text::Weight::NORMAL);

  let alignment = match input.align {
    TextAlignment::Left => cosmic_text::Align::Left,
    TextAlignment::Center => cosmic_text::Align::Center,
    TextAlignment::Right => cosmic_text::Align::Right,
  };

  // Add some text!
  buffer.set_text(
    &mut system.system,
    &input.content,
    &attrs,
    cosmic_text::Shaping::Advanced,
    Some(alignment),
  );

  // Perform shaping as desired
  buffer.shape_until_scroll(&mut system.system, true);

  let mut unique_glyphs = FastHashSet::default();
  let mut glyph_buffer: Vec<PositionedGlyph> = Vec::new();

  // Inspect the output runs
  for run in buffer.layout_runs() {
    for glyph in run.glyphs.iter() {
      let cache_key = glyph.physical((0., run.line_y), 1.0).cache_key;
      unique_glyphs.insert(cache_key);
      glyph_buffer.push(PositionedGlyph {
        glyph_key: cache_key,
        relative_x: glyph.x,
        relative_y: glyph.y - run.line_y,
      });
    }
  }

  for cache_key in &unique_glyphs {
    system
      .slug_glyph_cache
      .entry(*cache_key)
      .or_insert_with(|| {
        if let Some(outline_cmds) = system
          .swash
          .get_outline_commands(&mut system.system, *cache_key)
        {
          let mut curves = Vec::new();
          if let Some(bounds) = extract_curves(&outline_cmds, &mut curves) {
            let bands = build_bands(&curves, bounds, 8);

            let mut slug = SlugGlyph {
              glyph_key: *cache_key,
              curves,
              bands,
              bounds,
            };

            slug.sort();

            Some(slug)
          } else {
            None
          }
        } else {
          log::warn!(
            "unable to get outline commands of glyph {}",
            cache_key.glyph_id
          );
          None
        }
      });
  }

  SlugBuffer {
    positions: glyph_buffer,
    unique_glyphs,
    scale: input.scale,
  }
}

#[derive(Clone)]
pub struct GlyphBands {
  pub h_bands: Vec<Vec<u32>>,
  pub v_bands: Vec<Vec<u32>>,
  pub h_band_count: u32,
  pub v_band_count: u32,
}

/// Organize curves into horizontal and vertical bands.
fn build_bands(curves: &[f32], bounds: Box2, band_count: u32) -> GlyphBands {
  let x_min = bounds.min.x;
  let y_min = bounds.min.y;
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

#[derive(Clone)]
pub struct SlugGlyph {
  pub glyph_key: CacheKey,
  pub curves: Vec<f32>,
  pub bands: GlyphBands,
  pub bounds: Box2,
}

impl SlugGlyph {
  /// Sort curves: h-bands by descending max x, v-bands by descending max y
  pub fn sort(&mut self) {
    fn curve_axis_max(curves: &[f32], curve_index: u32, axis: usize) -> f32 {
      let base = curve_index as usize * 6;
      curves[base + axis]
        .max(curves[base + axis + 2])
        .max(curves[base + axis + 4])
    }

    for h_band in &mut self.bands.h_bands {
      h_band.sort_by(|&a, &b| {
        let ca = curve_axis_max(&self.curves, a, 0);
        let cb = curve_axis_max(&self.curves, b, 0);
        cb.total_cmp(&ca)
      });
    }
    for v_band in &mut self.bands.v_bands {
      v_band.sort_by(|&a, &b| {
        let ca = curve_axis_max(&self.curves, a, 1);
        let cb = curve_axis_max(&self.curves, b, 1);
        cb.total_cmp(&ca)
      });
    }
  }
}

// todo, https://github.com/diffusionstudio/slug-webgpu/commit/12d8bdf333263c1340a7ca5de0c36f7f90721be0
fn extract_curves(cmds: &[cosmic_text::Command], curves: &mut Vec<f32>) -> Option<Box2> {
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

  let mut bound = Box2::empty();
  let new_points = curves[start..].as_chunks::<2>().0;
  for [x, y] in new_points {
    bound.expand_by_point(Vec2::new(*x, *y));
  }

  Some(bound)
}
