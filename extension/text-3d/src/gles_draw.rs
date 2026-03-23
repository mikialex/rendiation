use rendiation_scene_rendering_gpu_base::*;
use rendiation_scene_rendering_gpu_gles::GLESModelRenderImpl;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::{slug_shader::slug_render, *};

pub fn use_text3d_gles_renderer(cx: &mut QueryGPUHookCx) -> Option<Text3dGlesRenderer> {
  let (cx, font_system) = cx.use_sharable_plain_state(|| FontSystem::new());

  let text3d_resources = cx.use_shared_hash_map("text3d gles gpu resources");

  let gpu = cx.gpu.clone();
  let font_system = font_system.clone();

  maintain_shared_map(
    &text3d_resources,
    cx.use_changes::<Text3dContent>().filter_map_changes(|t| t),
    |info| create_gles_render_data(&info, &mut *font_system.write(), &gpu), /* todo move out locking */
  );

  cx.when_render(|| Text3dGlesRenderer {
    access: global_database().read_foreign_key(),
    texts: text3d_resources.make_read_holder(),
  })
}

fn create_gles_render_data(
  data: &Text3dContentInfo,
  font_system: &mut FontSystem,
  gpu: &GPU,
) -> SlugTextGPUData {
  prepare_text(font_system, data).create_gpu(gpu)
}

pub struct Text3dGlesRenderer {
  access: ForeignKeyReadView<SceneModelText3dPayload>,
  texts: SharedHashMapRead<u32, SlugTextGPUData>,
}

impl GLESModelRenderImpl for Text3dGlesRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let text_id = self.access.get(idx)?;
    let text_gpu = self.texts.get(&text_id.alloc_index()).unwrap();

    let cmd = text_gpu.draw_command();

    Some((Box::new(text_gpu), cmd))
  }
  fn material_renderable<'a>(
    &'a self,
    _idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    Some(Box::new(())) // no material
  }
}

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
        data: bytemuck::cast_slice(&self.packed.curve_tex_data).to_vec(),
        format: TextureFormat::Rgba32Float,
        size: Size::from_u32_pair_min_one((TEX_WIDTH as u32, self.packed.curve_tex_height as u32)),
      },
    );

    let band_tex_data = create_gpu_texture2d(
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

struct SlugTextGPUData {
  pub indices: GPUBufferResourceView,
  pub vertices: GPUBufferResourceView,
  pub curve_tex_data: GPU2DTextureView,
  pub band_tex_data: GPUTypedTextureView<TextureDimension2, u32>,
}

impl SlugTextGPUData {
  pub fn draw_command(&self) -> DrawCommand {
    DrawCommand::Indexed {
      instances: 0..1,
      base_vertex: 0,
      indices: 0..u64::from(self.indices.view_byte_size()) as u32 / 4,
    }
  }
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

      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _binding| {
      let pos = builder.query::<SlugTextGlesVertexPos>();
      let tex = builder.query::<SlugTextGlesVertexTex>();
      // let jac = builder.query::<SlugTextGlesVertexJac>();

      // let viewport_size = builder.query::<ViewportRenderBufferSize>();
      // let dilated = slug_dilate(pos, tex, jac, todo!(), todo!(), todo!(), viewport_size).expand();

      // let local_position = vec3_node((dilated.vpos, val(0.)));

      // let object_world_position = builder.query::<WorldPositionHP>();
      // let (clip_position, _) = camera_transform_impl(builder, local_position, object_world_position);

      // builder.set_vertex_out::<FragmentUv>(dilated.texcoord);

      let local_position = vec3_node((pos.xy(), val(0.)));
      let object_world_position = builder.query::<WorldPositionHP>();
      let (clip_position, _) =
        camera_transform_impl(builder, local_position, object_world_position);
      builder.set_vertex_out::<FragmentUv>(tex.xy());
      builder.register::<ClipPosition>(clip_position);

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

      let coverage = slug_render(
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
