use crate::{ColorAttachment, PassContent, Scene};

use rendiation_algebra::Vec4;
use rendiation_webgpu::{BindGroup, PipelineBuilder, UniformBuffer};

pub struct HighLighter {
  pub color: Vec4<f32>,
}

impl Default for HighLighter {
  fn default() -> Self {
    Self {
      color: (0., 0.8, 1., 1.).into(),
    }
  }
}

impl HighLighter {
  pub fn draw(&self, mask: ColorAttachment) -> HighLightComposeTask {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
  }
}

pub struct HighLightComposeTask<'a> {
  mask: ColorAttachment,
  lighter: &'a HighLighter,
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut Scene,
    resource: &mut crate::ResourcePoolImpl,
    pass_info: &rendiation_webgpu::RenderPassInfo,
  ) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut rendiation_webgpu::GPURenderPass<'a>, scene: &'a Scene) {
    todo!()
  }
}

struct HighLightComposer {
  buffer: UniformBuffer<Vec4<f32>>,
  bindgroup: BindGroup,
}

impl HighLightComposer {
  fn build_pipeline(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    let mut builder = PipelineBuilder::default();
    builder.shader_source = format!(
      "
     {object_header}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] uv: vec2<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        [[builtin(vertex_index)]] vertex_index: u32;
      ) -> VertexOutput {{
        var out: VertexOutput;

        switch (i32(input.vertex_index)) {{
            case 0: {{
                pos = vec2<f32>(left, top);
                out.position = input.tex_left_top;
            }}
            case 1: {{
                pos = vec2<f32>(right, top);
                out.position = vec2<f32>(input.tex_right_bottom.x, input.tex_left_top.y);
            }}
            case 2: {{
                pos = vec2<f32>(left, bottom);
                out.position = vec2<f32>(input.tex_left_top.x, input.tex_right_bottom.y);
            }}
            case 3: {{
                pos = vec2<f32>(right, bottom);
                out.position = input.tex_right_bottom;
            }}
        }}

        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return textureSample(r_color, r_sampler, in.uv);
      }}
    ",
      object_header = ""
    );

    builder.build(device)
  }
}

pub struct HighLightDrawMaskTask<T> {
  object: T,
}

pub fn highlight<T>(object: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { object }
}

impl<T> PassContent for HighLightDrawMaskTask<T> {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut Scene,
    resource: &mut crate::ResourcePoolImpl,
    pass_info: &rendiation_webgpu::RenderPassInfo,
  ) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut rendiation_webgpu::GPURenderPass<'a>, scene: &'a Scene) {
    todo!()
  }
}
