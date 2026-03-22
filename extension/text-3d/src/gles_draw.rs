use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::{slug_shader::SlugRender, *};

pub fn use_text3d_gles_renderer(cx: &mut QueryGPUHookCx) -> Option<Text3dGlesRenderer> {
  todo!()
}

pub struct Text3dGlesRenderer {}

impl SlugTextPrepared {
  fn create_gpu(&self, gpu: &GPU) -> SlugTextGPUData {
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

    let curve_tex_data = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: bytemuck::cast_slice(&self.packed.curveTexData).to_vec(),
        format: TextureFormat::Rgba32Float,
        size: Size::from_u32_pair_min_one((TEX_WIDTH as u32, self.packed.curveTexHeight as u32)),
      },
    );

    let band_tex_data = create_gpu_texture2d(
      gpu,
      &GPUBufferImage {
        data: bytemuck::cast_slice(&self.packed.bandTexData).to_vec(),
        format: TextureFormat::Rgba32Uint,
        size: Size::from_u32_pair_min_one((TEX_WIDTH as u32, self.packed.bandTexHeight as u32)),
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

struct SlugTextGPUData {
  pub indices: GPUBufferResourceView,
  pub vertices: GPUBufferResourceView,
  pub curve_tex_data: GPU2DTextureView,
  pub band_tex_data: GPUTypedTextureView<TextureDimension2, u32>,
}

impl ShaderHashProvider for SlugTextGPUData {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for SlugTextGPUData {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.set_vertex_buffer_by_buffer_resource_view_next(&self.vertices);
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(&self.indices, IndexFormat::Uint32);
  }
}

impl GraphicsShaderProvider for SlugTextGPUData {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      // builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
      // builder.register_vertex::<WideLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      // let uv = builder.query::<GeometryUV>();
      // let color_with_alpha = builder.query::<GeometryColorWithAlpha>();
      // let uniform = binding.bind_by(&self.uniform).load().expand();

      // wide_line_vertex(
      //   builder.query::<WideLineStart>(),
      //   builder.query::<WideLineEnd>(),
      //   builder.query::<GeometryPosition>(),
      //   builder.query::<ViewportRenderBufferSize>(),
      //   uniform.width,
      //   builder,
      // );

      // builder.set_vertex_out::<FragmentUv>(uv);
      // builder.set_vertex_out::<DefaultDisplay>(color_with_alpha);
    });

    builder.fragment(|builder, binding| {
      builder.insert_type_tag::<UnlitMaterialTag>();

      let uv = builder.query::<FragmentUv>();
      let curve_tex_data = binding.bind_by(&self.curve_tex_data);
      let band_tex_data = binding.bind_by(&self.band_tex_data);

      let coverage = SlugRender(curve_tex_data, band_tex_data, uv, todo!(), todo!());
      // band_t_transform,
      // glyph_data,

      builder
        .register::<DefaultDisplay>(val(Vec4::new(0., 0., 0., 1.)) * coverage.splat::<Vec4<f32>>());
      //
    })
  }
}

// #[repr(C)]
// #[derive(rendiation_shader_api::ShaderVertex)]
// #[derive(Serialize, Deserialize)]
// #[derive(Clone, Copy, Debug, PartialEq, Default)]
// pub struct TextGlesVertex {
//   #[semantic(GeometryPosition)]
//   pub position: Vec3<f32>,

//   #[semantic(GeometryNormal)]
//   pub normal: Vec3<f32>,

//   #[semantic(GeometryUV)]
//   pub uv: Vec2<f32>,
// }
