use std::{any::TypeId, hash::Hash, rc::Rc};

use rendiation_texture::TextureSampler;
use rendiation_webgpu::{BindGroupDescriptor, GPUTexture2d, GPU};
use shadergraph::{FragmentUv, ShaderGraphProvider, SB};

use crate::{AttachmentReadView, PassContent, Scene, SceneRenderPass, ShaderPassBuilder};

pub struct CopyFrame<'a> {
  source: AttachmentReadView<'a>,
}

pub fn copy_frame(source: AttachmentReadView) -> CopyFrame {
  CopyFrame { source }
}

impl<'a> ShaderPassBuilder for CopyFrame<'a> {
  fn setup_pass(&self, ctx: &mut rendiation_webgpu::GPURenderPassCtx) {
    ctx.binding.setup_uniform(todo!(), SB::Material);
    ctx.binding.setup_pass(ctx.pass, &ctx.gpu.device, todo!());
  }
}

impl<'a> ShaderGraphProvider for CopyFrame<'a> {
  fn build(
    &self,
    builder: &mut shadergraph::ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), shadergraph::ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding
        .uniform_by(&self.lighter.data, SB::Material)
        .expand();

      let uv = builder.query::<FragmentUv>()?;
      builder.set_fragment_out(0, (uniform.color, edge_intensity(uv)))
    })
  }
}

// impl PassContent for CopyFrame {
//   fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
//     let resources = &mut scene.resources.content;
//     let bindgroup = gpu.device.create_bind_group(&BindGroupDescriptor {
//       layout: &Self::layout(&gpu.device),
//       entries: &[
//         wgpu::BindGroupEntry {
//           binding: 0,
//           resource: self.source.as_bindable(),
//         },
//         wgpu::BindGroupEntry {
//           binding: 1,
//           resource: resources
//             .samplers
//             .retrieve(&gpu.device, &TextureSampler::default())
//             .as_bindable(),
//         },
//       ],
//       label: None,
//     });
//     self.bindgroup = Some(bindgroup);

//     let mut hasher = Default::default();

//     let pass_info = ctx.pass_info;

//     TypeId::of::<Self>().hash(&mut hasher);
//     pass_info.format_info.hash(&mut hasher);

//     self.pipeline = resources
//       .pipeline_resource
//       .get_or_insert_with(hasher, || {
//         let mut builder = PipelineBuilder::default();

//         full_screen_vertex_shader(
//           &mut builder,
//           wgpu::BlendState::ALPHA_BLENDING.into(),
//           &pass_info.format_info,
//         );

//         builder
//           .with_layout::<Self>(&resources.layouts, &gpu.device)
//           .include_fragment_entry(
//             "
//           [[stage(fragment)]]
//           fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
//             return textureSample(texture, tex_sampler, in.uv);
//           }}
//           ",
//           )
//           .use_fragment_entry("fs_main");

//         builder.build(&gpu.device)
//       })
//       .clone()
//       .into();
//   }

//   fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, _scene: &'a Scene) {
//     pass.set_pipeline(self.pipeline.as_ref().unwrap());
//     pass.set_bind_group(0, self.bindgroup.as_ref().unwrap(), &[]);
//     pass.draw(0..4, 0..1);
//   }
// }
