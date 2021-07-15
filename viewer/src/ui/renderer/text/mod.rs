mod cache;
use std::borrow::Cow;

mod pipeline;
use pipeline::*;
mod text_quad_instance;
use text_quad_instance::*;

use glyph_brush::{
  ab_glyph::{self, Rect},
  BrushAction, BrushError, DefaultSectionHasher, Extra, FontId, GlyphBrushBuilder, GlyphCruncher,
  GlyphPositioner, Section, SectionGlyph,
};
use std::future::Future;

pub struct GPUxUITextPrimitive {
  vertex_buffer: wgpu::Buffer,
  length: u32,
}

pub struct TextRenderer {
  pipeline: Pipeline,
  glyph_brush: glyph_brush::GlyphBrush<Instance, Extra, ab_glyph::FontArc, DefaultSectionHasher>,
}

impl TextRenderer {
  pub fn new(
    device: &wgpu::Device,
    filter_mode: wgpu::FilterMode,
    render_format: wgpu::TextureFormat,
    // font_path: &str,
    // raw_builder: glyph_brush::GlyphBrushBuilder<F, H>,
  ) -> Self {
    // Prepare glyph_brush
    let inconsolata = ab_glyph::FontArc::try_from_slice(include_bytes!(
      "C:/Users/mk/Desktop/Inconsolata-Regular.ttf"
    ))
    .unwrap();

    let glyph_brush = GlyphBrushBuilder::using_font(inconsolata).build();

    let (cache_width, cache_height) = glyph_brush.texture_dimensions();
    Self {
      pipeline: Pipeline::new(
        device,
        filter_mode,
        render_format,
        cache_width,
        cache_height,
      ),
      glyph_brush,
    }
  }

  pub fn draw_gpu_tex(&self, pass: &mut wgpu::RenderPass, text: &GPUxUITextPrimitive) {}

  pub fn create_gpu_text<'a>(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    section: Section<'a, Extra>,
  ) -> Option<GPUxUITextPrimitive> {
    self.glyph_brush.queue(section);
    self.process_queued(device, encoder)
  }

  fn process_queued(
    &mut self,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
  ) -> Option<GPUxUITextPrimitive> {
    let mut brush_action;

    loop {
      brush_action = self.glyph_brush.process_queued(
        |rect, tex_data| {
          let offset = [rect.min[0] as u16, rect.min[1] as u16];
          let size = [rect.width() as u16, rect.height() as u16];

          self
            .pipeline
            .update_cache(device, encoder, offset, size, tex_data);
        },
        Instance::from_vertex,
      );

      match brush_action {
        Ok(brush_action) => match brush_action {
          BrushAction::Draw(verts) => {
            self.pipeline.create_gpu_text(device, &verts);
          }
          BrushAction::ReDraw => {}
        },
        Err(BrushError::TextureTooSmall { suggested }) => {
          // TODO: Obtain max texture dimensions using `wgpu`
          // This is currently not possible I think. Ask!
          let max_image_dimension = 2048;

          let (new_width, new_height) = if (suggested.0 > max_image_dimension
            || suggested.1 > max_image_dimension)
            && (self.glyph_brush.texture_dimensions().0 < max_image_dimension
              || self.glyph_brush.texture_dimensions().1 < max_image_dimension)
          {
            (max_image_dimension, max_image_dimension)
          } else {
            suggested
          };

          log::warn!(
            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
            old = self.glyph_brush.texture_dimensions(),
            new = (new_width, new_height),
          );

          self
            .pipeline
            .increase_cache_size(device, new_width, new_height);
          self.glyph_brush.resize_texture(new_width, new_height);
        }
      }
    }
  }
}

/// Helper function to generate a generate a transform matrix.
pub fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
  #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, -2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}
