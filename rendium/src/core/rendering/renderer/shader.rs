// use render_target::{RenderTarget, TargetStatesProvider};
// use rendiation_webgpu::*;
// use rendiation_webgpu_derives::BindGroup;
// use rendiation_renderable_mesh::geometry::IndexedGeometry;

// pub struct QuadShading {
//   pub pipeline: WGPUPipeline,
// }

// #[derive(BindGroup)]
// pub struct QuadShadingParam<'a> {
//   #[bind_type = "uniform-buffer:vertex"]
//   pub transform: &'a WGPUBuffer,

//   #[bind_type = "uniform-buffer:fragment"]
//   pub color: &'a WGPUBuffer,
// }

// impl QuadShading {
//   pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
//     let pipeline = PipelineBuilder::new(
//       renderer,
//       load_glsl(include_str!("./quad.vert"), ShaderType::Vertex),
//       load_glsl(include_str!("./quad.frag"), ShaderType::Fragment),
//     )
//     .as_mut()
//     .geometry::<IndexedGeometry>()
//     .binding_group::<QuadShadingParam>()
//     .target_states(target.create_target_states().as_ref())
//     .build();
//     Self { pipeline }
//   }
// }

// pub struct CopyShading {
//   pub pipeline: WGPUPipeline,
// }

// impl CopyShading {
//   pub fn new(renderer: &WGPURenderer, target: &impl TargetStatesProvider) -> Self {
//     let pipeline = PipelineBuilder::new(
//       renderer,
//       load_glsl(include_str!("./copy.vert"), ShaderType::Vertex),
//       load_glsl(include_str!("./copy.frag"), ShaderType::Fragment),
//     )
//     .as_mut()
//     .geometry::<IndexedGeometry>()
//     .binding_group::<CopyShadingParam>()
//     .target_states(target.create_target_states().as_mut().first_color(|s| {
//       s.color_blend(wgpu::BlendState {
//         src_factor: BlendFactor::SrcAlpha,
//         dst_factor: BlendFactor::OneMinusSrcAlpha,
//         operation: BlendOperation::Add,
//       })
//     }))
//     .build();
//     Self { pipeline }
//   }
// }

// #[derive(BindGroup)]
// pub struct CopyShadingParam<'a> {
//   #[bind_type = "texture2d:fragment"]
//   pub texture_view: &'a wgpu::TextureView,

//   #[bind_type = "sampler:fragment"]
//   pub sampler: &'a WGPUSampler,
// }
