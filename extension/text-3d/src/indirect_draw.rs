use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_webgpu_midc_downgrade::require_midc_downgrade;

use crate::*;

// todo, the glyph data between the different text are not shared
pub fn use_text3d_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  font_system: &Arc<RwLock<FontSystem>>,
  force_midc_downgrade: bool,
) -> Option<Text3dIndirectRenderer> {
  let sm_to_text3d_device = use_db_device_foreign_key::<SceneModelText3dPayload>(cx);

  let slug_buffer = cx.use_shared_dual_query(Text3dSlugBuffer(font_system.clone()));

  let font_system = font_system.clone();

  let indirect_resource = slug_buffer.map_spawn_stage_in_thread_dual_query(cx, move |q| {
    let font_system = font_system.make_read_holder();
    q.delta
      .into_change()
      .collective_map(move |slug_buffer| {
        ExternalRefPtr::new(prepare_indirect_text(&slug_buffer, &font_system))
      })
      .materialize()
  });

  let (r, r2) = indirect_resource.fork();
  let (rr, rrr) = r2.fork();

  // todo: avoid extra clone
  let curve_source =
    r.map_changes(|v| ExternalRefPtr::new(cast_slice::<_, u8>(v.curves.as_slice()).to_vec()));
  let band_source =
    rr.map_changes(|v| ExternalRefPtr::new(cast_slice::<_, u8>(v.bands.as_slice()).to_vec()));
  let vertices_source =
    rrr.map_changes(|v| ExternalRefPtr::new(cast_slice::<_, u8>(v.vertices.as_slice()).to_vec()));

  let (curves, curve_range_updates) =
    use_range_allocated_device_buffers(cx, "text_buffer curves", 1024, u32::MAX, curve_source);
  let (bands, band_range_updates) =
    use_range_allocated_device_buffers(cx, "text_buffer bands", 1024, u32::MAX, band_source);
  let (vertices, vertices_range_updates) =
    use_range_allocated_device_buffers(cx, "text_buffer vertices", 1024, u32::MAX, vertices_source);

  let (cx, params) = cx.use_storage_buffer_with_host_backup::<TextMeta>(
    "wide line buffer parameters and range info",
    128,
    u32::MAX,
  );

  let offset = std::mem::offset_of!(TextMeta, text_curves_range);
  let curve_range_updates = curve_range_updates.map(|a| a.allocation_changes.clone());
  curve_range_updates.update_storage_array_with_host(cx, params, offset);

  let offset = std::mem::offset_of!(TextMeta, text_band_range);
  let band_range_updates = band_range_updates.map(|a| a.allocation_changes.clone());
  band_range_updates.update_storage_array_with_host(cx, params, offset);

  let offset = std::mem::offset_of!(TextMeta, text_vertices_range);
  let vertices_range_updates = vertices_range_updates.map(|a| a.allocation_changes.clone());
  vertices_range_updates.update_storage_array_with_host(cx, params, offset);

  params.use_max_item_count_by_db_entity::<Text3dEntity>(cx);
  params.use_update(cx);

  cx.when_render(|| Text3dIndirectRenderer {
    access: global_database().read_foreign_key(),
    curves,
    bands,
    vertices,
    text_meta: params.get_gpu_buffer(),
    text_meta_host: params.buffer.make_read_holder(),
    sm_to_text3d_device: sm_to_text3d_device.unwrap(),
    used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
  })
}

pub struct Text3dIndirectRenderer {
  curves: AbstractReadonlyStorageBuffer<[CurveData]>,
  bands: AbstractReadonlyStorageBuffer<[u32]>,
  vertices: AbstractReadonlyStorageBuffer<[TextGlyphQuad]>,

  /// text id -> start of text data
  text_meta: AbstractReadonlyStorageBuffer<[TextMeta]>,
  /// host is require to support host side draw command generation
  text_meta_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<TextMeta>>,
  /// sm id -> text id
  sm_to_text3d_device: AbstractReadonlyStorageBuffer<[u32]>,
  /// sm id -> text id, host version
  access: ForeignKeyReadView<SceneModelText3dPayload>,
  used_in_midc_downgrade: bool,
}

impl IndirectModelRenderImpl for Text3dIndirectRenderer {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.access.get(any_id)?;
    Some(())
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    self.access.get(any_idx)?;
    Some(Box::new(()))
  }

  fn shape_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    self.access.get(any_idx)?;

    let c = Text3dIndirectRender {
      curves: &self.curves,
      bands: &self.bands,
      vertices: &self.vertices,
      text_meta: &self.text_meta,
      sm_to_text3d_device: &self.sm_to_text3d_device,
    };

    Some(Box::new(c))
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    self.access.get(batch.impl_select_id)?;

    let draw_command_builder = self
      .make_draw_command_builder(batch.impl_select_id)
      .unwrap();

    ctx
      .access_parallel_compute(|cx| {
        batch.create_default_indirect_draw_provider(
          draw_command_builder,
          cx,
          self.used_in_midc_downgrade,
        )
      })
      .into()
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    self.access.get(any_idx)?;

    let creator = Text3dDrawCreator {
      params: self.text_meta.clone(),
      params_host: self.text_meta_host.clone(),
      sm_to_text_device: self.sm_to_text3d_device.clone(),
      sm_to_text: self.access.clone(),
    };

    DrawCommandBuilder::NoneIndexed(Box::new(creator)).into()
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    _any_idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    Some(Box::new(())) // no material
  }
}

#[derive(Clone)]
struct Text3dDrawCreator {
  params: AbstractReadonlyStorageBuffer<[TextMeta]>,
  sm_to_text_device: AbstractReadonlyStorageBuffer<[u32]>,
  sm_to_text: ForeignKeyReadView<SceneModelText3dPayload>,
  params_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<TextMeta>>,
}

impl ShaderHashProvider for Text3dDrawCreator {
  shader_hash_type_id! {}
}

impl NoneIndexedDrawCommandBuilder for Text3dDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let model = self.sm_to_text.get(id).unwrap();
    let param = self.params_host.get(model.alloc_index()).unwrap();
    let seg_count = param.text_vertices_range.y / TextGlyphQuad::u32_size();

    if param.text_vertices_range.x == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
      return None;
    }

    DrawCommand::Array {
      instances: 0..1,
      vertices: 0..6 * seg_count,
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let params = cx.bind_by(&self.params);
    let sm_to_wide_line_device = cx.bind_by(&self.sm_to_text_device);
    Box::new(DrawCmdBuilderInvocation {
      params,
      sm_to_text_device: sm_to_wide_line_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.params);
    builder.bind(&self.sm_to_text_device);
  }
}

struct DrawCmdBuilderInvocation {
  params: ShaderReadonlyPtrOf<[TextMeta]>,
  sm_to_text_device: ShaderReadonlyPtrOf<[u32]>,
}

impl NoneIndexedDrawCommandBuilderInvocation for DrawCmdBuilderInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndirectArgsStorage> {
    let text_id = self.sm_to_text_device.index(draw_id).load();
    // the implementation of range allocate assure the count is zero if allocation failed
    let seg_count =
      self.params.index(text_id).text_vertices_range().load().y() / val(TextGlyphQuad::u32_size());

    ENode::<DrawIndirectArgsStorage> {
      vertex_count: val(6) * seg_count,
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}

struct Text3dIndirectRender<'a> {
  curves: &'a AbstractReadonlyStorageBuffer<[CurveData]>,
  bands: &'a AbstractReadonlyStorageBuffer<[u32]>,
  vertices: &'a AbstractReadonlyStorageBuffer<[TextGlyphQuad]>,

  /// text id -> start of text data
  text_meta: &'a AbstractReadonlyStorageBuffer<[TextMeta]>,
  /// sm id -> text id
  sm_to_text3d_device: &'a AbstractReadonlyStorageBuffer<[u32]>,
}

both!(Text3DShaderId, u32);
both!(Text3DBandEntry, u32);
both!(Text3DBandMax, Vec2<u32>);

impl<'a> GraphicsShaderProvider for Text3dIndirectRender<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let sm_id = builder.query::<LogicalRenderEntityId>();
      let sm_to_text3d_device = binding.bind_by(self.sm_to_text3d_device);
      let text_id = sm_to_text3d_device.index(sm_id).load();
      builder.set_vertex_out::<Text3DShaderId>(text_id);
      builder.register::<Text3DShaderId>(text_id);

      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let mut text_meta = BindingPreparer::new(self.text_meta);

    builder.vertex(|builder, binding| {
      let text_id = builder.query::<Text3DShaderId>();

      let text_meta = text_meta.using(binding);
      let text_meta = text_meta.index(text_id).load().expand();

      let vertex_index = builder.query::<VertexIndex>();
      let instance_index = vertex_index / val(6);
      let vertex_index = vertex_index % val(6);

      let vertex_offset = text_meta.text_vertices_range.x() / val(TextGlyphQuad::u32_size());
      let vertices = binding.bind_by(self.vertices);
      let quad = vertices.index(instance_index + vertex_offset);
      let quad = quad.load().expand();

      let offset = val(Vec2::new(0., 0.)).make_local_var();
      switch_by(vertex_index)
        .case(0, || offset.store(Vec2::new(0., 0.)))
        .case(1, || offset.store(Vec2::new(1., 1.)))
        .case(2, || offset.store(Vec2::new(1., 0.)))
        .case(3, || offset.store(Vec2::new(0., 0.)))
        .case(4, || offset.store(Vec2::new(0., 1.)))
        .case(5, || offset.store(Vec2::new(1., 1.)))
        .end_with_default(|| {});
      let offset = offset.load();

      let pos = quad.obj_space_min + quad.obj_space_size * offset;
      let uv = quad.em_space_min + quad.em_space_size * offset;
      let local_position = vec3_node((pos, val(0.)));
      let object_world_position = builder.query::<WorldPositionHP>();
      let (clip_position, render_space_position) =
        camera_transform_impl(builder, local_position, object_world_position);
      builder.register::<ClipPosition>(clip_position);
      builder.register::<VertexRenderPosition>(render_space_position);

      builder.set_vertex_out::<FragmentUv>(uv);

      builder.set_vertex_out::<Text3DBandEntry>(quad.band_offset + text_meta.text_band_range.x());
      builder.set_vertex_out::<Text3DBandMax>(quad.band_max);
      builder.set_vertex_out_with_given_interpolate::<Text3dBandTransform>(
        quad.band_transform,
        ShaderInterpolation::Flat,
      );
    });

    builder.fragment(|builder, binding| {
      let text_id = builder.query::<Text3DShaderId>();
      let text_meta = text_meta.using(binding);
      let curve_text_global_offset = text_meta
        .index(text_id)
        .load()
        .expand()
        .text_curves_range
        .x();

      builder.insert_type_tag::<UnlitMaterialTag>();

      let uv = builder.query::<FragmentUv>();
      let curves = binding.bind_by(self.curves);
      let bands = binding.bind_by(self.bands);

      let band_t_transform = builder.query::<Text3dBandTransform>();
      let band_max = builder.query::<Text3DBandMax>();
      let band_offset = builder.query::<Text3DBandEntry>();

      let coverage = IndirectSlugShaderDataSource {
        bands,
        curves,
        glyph_abs_band_offset: band_offset,
        curve_text_global_offset,
        band_t_transform,
        band_max,
      }
      .slug_render(uv, val(0));

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
    });
  }
}

impl<'a> ShaderHashProvider for Text3dIndirectRender<'a> {
  shader_hash_type_id! {Text3dIndirectRender<'static>}
}

impl<'a> ShaderPassBuilder for Text3dIndirectRender<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.sm_to_text3d_device);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.text_meta);
    ctx.binding.bind(self.vertices);
    ctx.binding.bind(self.curves);
    ctx.binding.bind(self.bands);
  }
}

struct IndirectSlugShaderDataSource {
  bands: ShaderReadonlyPtrOf<[u32]>,
  curves: ShaderReadonlyPtrOf<[CurveData]>,
  glyph_abs_band_offset: Node<u32>,
  curve_text_global_offset: Node<u32>,
  band_t_transform: Node<Vec4<f32>>,
  band_max: Node<Vec2<u32>>,
}

impl SlugShaderComputer for IndirectSlugShaderDataSource {
  fn get_band_data_source(
    &self,
    render_coord: Node<Vec2<f32>>,
  ) -> Box<dyn SlugShaderBandDataSource> {
    // Determine what bands the current pixel lies in by applying a scale and offset
    // to the render coordinates. The scales are given by bandTransform.xy, and the
    // offsets are given by bandTransform.zw. Band indexes are clamped to [0, bandMax.xy].
    let band_t_transform = self.band_t_transform;
    let band_index = (render_coord * band_t_transform.xy() + band_t_transform.zw())
      .into_i32()
      .clamp(val(Vec2::zero()), self.band_max.into_i32());

    let band_h = band_index.x().into_u32();
    let band_v = band_index.y().into_u32();

    let band_h_count = self.bands.index(self.glyph_abs_band_offset).load();
    let band_v_count = self.bands.index(self.glyph_abs_band_offset + val(1)).load();
    let header_len = (band_h_count + band_v_count) * val(2) + val(2);

    let h_info_offset = self.glyph_abs_band_offset + val(2) + band_h * val(2);
    let h_offset = self.bands.index(h_info_offset).load() + header_len;
    let h_count = self.bands.index(h_info_offset + val(1)).load() + header_len;

    let v_info_offset =
      self.glyph_abs_band_offset + val(2) + band_h_count * val(2) + band_v * val(2);
    let v_offset = self.bands.index(v_info_offset).load() + header_len;
    let v_count = self.bands.index(v_info_offset + val(1)).load() + header_len;

    Box::new(IndirectSlugShaderBandDataSource {
      bands: self.bands.clone(),
      curves: self.curves.clone(),
      h_band_range: vec2_node((h_offset, h_count)),
      v_band_range: vec2_node((v_offset, v_count)),
      curve_text_global_offset: self.curve_text_global_offset,
    })
  }
}

struct IndirectSlugShaderBandDataSource {
  bands: ShaderReadonlyPtrOf<[u32]>,
  h_band_range: Node<Vec2<u32>>,
  v_band_range: Node<Vec2<u32>>,
  curves: ShaderReadonlyPtrOf<[CurveData]>,
  curve_text_global_offset: Node<u32>,
}

impl SlugShaderBandDataSource for IndirectSlugShaderBandDataSource {
  fn iter_curves_horizontal(
    &self,
    render_coord: Node<Vec2<f32>>,
  ) -> Box<dyn ShaderIterator<Item = (Node<Vec4<f32>>, Node<Vec2<f32>>)> + '_> {
    let iter = self
      .h_band_range
      .into_shader_iter()
      .map(move |curve_index: Node<u32>| {
        let curve_index = self.bands.index(curve_index).load();
        let abs_curve_idx = self.curve_text_global_offset + curve_index;
        let curve = self.curves.index(abs_curve_idx).load();
        let curve = curve.expand();
        (
          vec4_node((curve.p1 - render_coord, curve.p2 - render_coord)),
          curve.p3 - render_coord,
        )
      });
    Box::new(iter)
  }

  fn iter_curves_vertical(
    &self,
    render_coord: Node<Vec2<f32>>,
  ) -> Box<dyn ShaderIterator<Item = (Node<Vec4<f32>>, Node<Vec2<f32>>)> + '_> {
    let iter = self
      .v_band_range
      .into_shader_iter()
      .map(move |curve_index: Node<u32>| {
        let curve_index = self.bands.index(curve_index).load();
        let abs_curve_idx = self.curve_text_global_offset + curve_index;
        let curve = self.curves.index(abs_curve_idx).load();
        let curve = curve.expand();
        (
          vec4_node((curve.p1 - render_coord, curve.p2 - render_coord)),
          curve.p3 - render_coord,
        )
      });
    Box::new(iter)
  }
}
