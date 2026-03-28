use rendiation_scene_rendering_gpu_base::*;
use rendiation_scene_rendering_gpu_gles::GLESModelRenderImpl;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_text3d_gles_renderer(
  cx: &mut QueryGPUHookCx,
  font_system: &Arc<RwLock<FontSystem>>,
) -> Option<Text3dGlesRenderer> {
  let text3d_resources = cx.use_shared_hash_map("text3d gles gpu resources");

  let gpu = cx.gpu.clone();

  let slug_buffer = cx
    .use_shared_dual_query(Text3dSlugBuffer(font_system.clone()))
    .use_assure_result(cx);

  maintain_shared_map_avoid_unnecessary_creator_init(
    &text3d_resources,
    slug_buffer.into_delta_change(),
    || {
      let font_sys = font_system.make_read_holder();
      move |v| prepare_gles_text(&v, &font_sys).map(|v| v.create_gpu(&gpu))
    },
  );

  cx.when_render(|| Text3dGlesRenderer {
    access: global_database().read_foreign_key(),
    texts: text3d_resources.make_read_holder(),
  })
}

pub struct Text3dGlesRenderer {
  access: ForeignKeyReadView<SceneModelText3dPayload>,
  texts: SharedHashMapRead<RawEntityHandle, Option<SlugTextGPUData>>,
}

impl GLESModelRenderImpl for Text3dGlesRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let text_id = self.access.get(idx)?;
    let text_gpu = self.texts.get(text_id.raw_handle_ref()).unwrap();
    // todo, we should distinguish between this case from the error case
    let text_gpu = text_gpu.as_ref()?;

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

pub struct SlugTextGPUData {
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
      let jac = builder.query::<SlugTextGlesVertexJac>();

      let viewport_size = builder.query::<ViewportRenderBufferSize>();

      let object_world_position = builder.query::<WorldPositionHP>();
      let view_projection = builder.query::<CameraViewNoneTranslationProjectionMatrix>();

      let enable_dilation = false;

      if enable_dilation {
        let view_projection_t = view_projection.transpose();

        // let world_matrix = builder.query::<WorldNoneTranslationMatrix>();
        // // SlugDilate follows the reference vertex shader and expects the first, second,
        // // and fourth rows of the effective local-to-clip transform.
        // let local_to_clip = view_projection * world_matrix;
        // let local_to_clip_t = local_to_clip.transpose();
        // let render_origin =
        //   compute_render_space_position(builder, val(Vec3::zero()), object_world_position);
        // let clip_origin = view_projection * render_origin;

        // let m0 = local_to_clip_t.nth_colum(0);
        // let m1 = local_to_clip_t.nth_colum(1);
        // let m3 = local_to_clip_t.nth_colum(3);

        // let m0 = vec4_node((m0.x(), m0.y(), m0.z(), clip_origin.x()));
        // let m1 = vec4_node((m1.x(), m1.y(), m1.z(), clip_origin.y()));
        // let m3 = vec4_node((m3.x(), m3.y(), m3.z(), clip_origin.w()));

        let dilated = slug_dilate(
          pos,
          tex,
          jac,
          view_projection_t.nth_colum(0),
          view_projection_t.nth_colum(1),
          view_projection_t.nth_colum(3),
          viewport_size,
        )
        .expand();

        let dilated_local_position = vec3_node((dilated.vpos, val(0.)));
        let (clip_position, position_in_render_space) =
          camera_transform_impl(builder, dilated_local_position, object_world_position);

        builder.set_vertex_out::<FragmentUv>(dilated.texcoord);
        builder.register::<ClipPosition>(clip_position);

        builder.register::<VertexRenderPosition>(position_in_render_space);
      } else {
        let local_position = vec3_node((pos.xy(), val(0.)));
        let object_world_position = builder.query::<WorldPositionHP>();
        let (clip_position, render_space_position) =
          camera_transform_impl(builder, local_position, object_world_position);
        builder.set_vertex_out::<FragmentUv>(tex.xy());
        builder.register::<ClipPosition>(clip_position);

        builder.register::<VertexRenderPosition>(render_space_position);
      }

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

      builder.frag_output.iter_mut().for_each(|p| {
        if p.is_blendable() {
          p.states.blend = BlendState::ALPHA_BLENDING.into();
        }
      });
      if let Some(depth) = &mut builder.depth_stencil {
        depth.depth_write_enabled = false;
      }
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
