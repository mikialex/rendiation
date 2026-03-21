use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_scene_core::SceneModelEntity;
use rendiation_webgpu::GPU2DTextureView;

mod draw;
mod slug_shader;

pub fn register_text3d_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelText3dPayload>(sparse);

  global_database()
    .declare_entity::<Text3dEntity>()
    .declare_component::<Text3dContent>();
}

declare_foreign_key!(SceneModelText3dPayload, SceneModelEntity, Text3dEntity);

declare_entity!(Text3dEntity);
declare_component!(Text3dContent, Text3dEntity, ExternalRefPtr<String>);
declare_component!(Text3dFont, Text3dEntity, Option<u32>);
declare_component!(Text3dWeight, Text3dEntity, Option<u32>);
declare_component!(Text3dColor, Text3dEntity, Vec3<f32>, Vec3::zero());

pub struct FontSystem {
  system: cosmic_text::FontSystem,
  swash: cosmic_text::SwashCache,
}

impl FontSystem {
  fn build_text_slug_data(&mut self, text: &str) -> SlugTextGPUData {
    // Text metrics indicate the font size and line height of a buffer
    let metrics = cosmic_text::Metrics::new(14.0, 20.0);

    // A Buffer provides shaping and layout for a UTF-8 string, create one per text widget
    let mut buffer = cosmic_text::Buffer::new(&mut self.system, metrics);

    // Set a size for the text buffer, in pixels
    buffer.set_size(&mut self.system, Some(80.0), Some(25.0));

    // Attributes indicate what font to choose
    let attrs = cosmic_text::Attrs::new();

    // Add some text!
    buffer.set_text(
      &mut self.system,
      "Hello, Rust!\n Hello, World!",
      &attrs,
      cosmic_text::Shaping::Advanced,
      None,
    );

    // Perform shaping as desired
    buffer.shape_until_scroll(&mut self.system, true);

    let mut used_glyphs = FastHashSet::default();

    let mut curves = Vec::new();
    // let mut bounds = Vec::new();

    // Inspect the output runs
    for run in buffer.layout_runs() {
      for glyph in run.glyphs.iter() {
        println!("{:#?}", glyph);
        used_glyphs.insert(glyph.glyph_id);

        if let Some(outline_cmds) = self.swash.get_outline_commands(
          &mut self.system,
          glyph.physical((0., run.line_y), 1.0).cache_key,
        ) {
          extract_curves(&outline_cmds, &mut curves);
          // let bounds = self.swash.get_image(font_system, cache_key)
        }
      }
    }

    todo!()
  }
}

struct SlugTextGPUData {
  pub indices: Vec<u32>,
  pub vertices: Vec<f32>,
  pub curve_tex_data: GPU2DTextureView,
  pub band_tex_data: GPU2DTextureView,
}

struct Bounds {
  xMin: f32,
  yMin: f32,
  xMax: f32,
  yMax: f32,
}

impl Bounds {
  pub fn size(&self) -> (f32, f32) {
    (self.xMax - self.xMin, self.yMax - self.yMin)
  }
}

struct GlyphBands {
  hBands: Vec<Vec<u32>>,
  vBands: Vec<Vec<u32>>,
  hBandCount: u32,
  vBandCount: u32,
}

/// Organize curves into horizontal and vertical bands.
fn buildBands(curves: &[f32], bounds: Bounds, bandCount: u32) -> GlyphBands {
  let Bounds { xMin, yMin, .. } = bounds;
  let (width, height) = bounds.size();

  let mut hBands = Vec::new();
  for i in 0..bandCount {
    hBands.push(Vec::new());
  }

  let mut vBands = Vec::new();
  for i in 0..bandCount {
    vBands.push(Vec::new());
  }

  for (ci, [p0x, p0y, p1x, p1y, p2x, p2y]) in curves.as_chunks::<6>().0.iter().enumerate() {
    let ci = ci as u32;
    let cyMin = p0y.min(p1y.min(*p2y));
    let cyMax = p0y.max(p1y.max(*p2y));
    let cxMin = p0x.min(p1x.min(*p2x));
    let cxMax = p0x.max(p1x.max(*p2x));

    if height > 0. {
      let b0 = (cyMin - yMin) / height * bandCount as f32;
      let b0 = (b0.floor() as u32).max(0);

      let b1 = (cyMax - yMin) / height * bandCount as f32;
      let b1 = (b1.floor() as u32).min(bandCount - 1);
      for b in b0..=b1 {
        hBands[b as usize].push(ci);
      }
    }

    if width > 0. {
      let b0 = (cxMin - xMin) / width * bandCount as f32;
      let b0 = (b0.floor() as u32).max(0);

      let b1 = (cxMin - xMin) / width * bandCount as f32;
      let b1 = (b1.floor() as u32).min(bandCount - 1);

      for b in b0..=b1 {
        vBands[b as usize].push(ci);
      }
    }
  }

  GlyphBands {
    hBands,
    vBands,
    hBandCount: bandCount,
    vBandCount: bandCount,
  }
}

fn extract_curves(cmds: &[cosmic_text::Command], curves: &mut Vec<f32>) {
  let mut current_x = 0.0;
  let mut current_y = 0.0;

  let mut start_x = 0.0;
  let mut start_y = 0.0;

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
        //         const cx1 = cmd.x1, cy1 = cmd.y1;
        //         const cx2 = cmd.x2, cy2 = cmd.y2;
        //         const ex = cmd.x, ey = cmd.y;

        //         const m01x = (curX + cx1) / 2, m01y = (curY + cy1) / 2;
        //         const m12x = (cx1 + cx2) / 2, m12y = (cy1 + cy2) / 2;
        //         const m23x = (cx2 + ex) / 2, m23y = (cy2 + ey) / 2;
        //         const m012x = (m01x + m12x) / 2, m012y = (m01y + m12y) / 2;
        //         const m123x = (m12x + m23x) / 2, m123y = (m12y + m23y) / 2;
        //         const midx = (m012x + m123x) / 2, midy = (m012y + m123y) / 2;

        //         curves.push({
        //           p0x: curX, p0y: curY,
        //           p1x: m01x, p1y: m01y,
        //           p2x: midx, p2y: midy,
        //         });
        //         curves.push({
        //           p0x: midx, p0y: midy,
        //           p1x: m123x, p1y: m123y,
        //           p2x: ex, p2y: ey,
        //         });
        //         curX = ex;
        //         curY = ey;
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
  // sys.system.
}
