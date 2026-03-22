use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::{
  slug_shader::{slug_dilate, SlugRender},
  *,
};

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

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.curve_tex_data);
    ctx.binding.bind(&self.band_tex_data);
  }
}

both!(Text3dBandTransform, Vec4<f32>);
both!(Text3dGlyphData, Vec4<i32>);

impl GraphicsShaderProvider for SlugTextGPUData {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      builder.register_vertex::<TextGlesVertex>(VertexStepMode::Vertex);

      // builder.register::<GeometryPosition>(todo!());

      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      // builder.register::<GeometryUV>(todo!());
      // todo
      // builder.set_vertex_out::<FragmentUv>(uv);

      let viewport_size = builder.query::<ViewportRenderBufferSize>();

      let pos = builder.query::<SlugTextGlesVertexPos>();
      let tex = builder.query::<SlugTextGlesVertexTex>();
      let jac = builder.query::<SlugTextGlesVertexJac>();
      let dilated = slug_dilate(pos, tex, jac, todo!(), todo!(), todo!(), viewport_size).expand();
      builder.set_vertex_out::<FragmentUv>(dilated.texcoord);

      // builder.register::<ClipPosition>(clip);

      builder.register_vertex::<TextGlesVertex>(VertexStepMode::Vertex);

      let color_with_alpha = builder.query::<SlugTextGlesVertexCol>();
      builder.set_vertex_out::<DefaultDisplay>(color_with_alpha);

      let band = builder.query::<SlugTextGlesVertexBnd>();
      builder.set_vertex_out_with_given_interpolate::<Text3dBandTransform>(
        band,
        ShaderInterpolation::Flat,
      );

      let tex = builder.query::<SlugTextGlesVertexTex>();
      let g1 = tex.z().bitcast::<u32>();
      let g2 = tex.w().bitcast::<u32>();
      let g = vec2_node((g1, g2));
      let vgly = vec4_node((
        (g.x() & val(0xFFFF)).into_i32(),
        (g.x() >> val(16)).into_i32(),
        (g.y() & val(0xFFFF)).into_i32(),
        (g.y() >> val(16)).into_i32(),
      ));

      builder.set_vertex_out::<Text3dGlyphData>(vgly);
    });

    builder.fragment(|builder, binding| {
      builder.insert_type_tag::<UnlitMaterialTag>();

      let uv = builder.query::<FragmentUv>();
      let curve_tex_data = binding.bind_by(&self.curve_tex_data);
      let band_tex_data = binding.bind_by(&self.band_tex_data);

      let band_t_transform = builder.query::<Text3dBandTransform>();
      let glyph_data = builder.query::<Text3dGlyphData>();

      let coverage = SlugRender(
        curve_tex_data,
        band_tex_data,
        uv,
        band_t_transform,
        glyph_data,
      );

      builder
        .register::<DefaultDisplay>(val(Vec4::new(0., 0., 0., 1.)) * coverage.splat::<Vec4<f32>>());
      //
    })
  }
}

#[repr(C)]
#[derive(rendiation_shader_api::ShaderVertex)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct TextGlesVertex {
  #[semantic(SlugTextGlesVertexPos)]
  pub pos: Vec4<f32>,

  #[semantic(SlugTextGlesVertexTex)]
  pub tex: Vec4<f32>,

  #[semantic(SlugTextGlesVertexJac)]
  pub jac: Vec4<f32>,

  #[semantic(SlugTextGlesVertexBnd)]
  pub bnd: Vec4<f32>,

  #[semantic(SlugTextGlesVertexCol)]
  pub col: Vec4<f32>,
}

only_vertex!(SlugTextGlesVertexPos, Vec4<f32>);
only_vertex!(SlugTextGlesVertexTex, Vec4<f32>);
only_vertex!(SlugTextGlesVertexJac, Vec4<f32>);
only_vertex!(SlugTextGlesVertexBnd, Vec4<f32>);
only_vertex!(SlugTextGlesVertexCol, Vec4<f32>);
