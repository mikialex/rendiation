use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_webgpu_midc_downgrade::require_midc_downgrade;

use crate::*;

pub fn use_widen_styled_points_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  force_midc_downgrade: bool,
) -> Option<WideStyledPointsIndirectRenderer> {
  let data_source = cx
    .use_dual_query::<WidesStyledPointsMeshBuffer>()
    .map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
      source_info.delta().into_change().collective_map(|buffer| {
        // here we assume the buffer is correctly aligned
        let buffer: &[WideStyledPointVertex] = cast_slice(&buffer);
        let mut new_buffer =
          Vec::with_capacity(buffer.len() * std::mem::size_of::<WideStyledPointVertexStorage>());
        for v in buffer {
          new_buffer.extend_from_slice(
            WideStyledPointVertexStorage {
              position: v.position,
              style_id: v.style_id,
              width: v.width,
              ..Default::default()
            }
            .as_bytes(),
          );
        }
        ExternalRefPtr::new(new_buffer)
      })
    });

  let (points_buffer, allocation_info) = use_range_allocated_device_buffers::<
    WideStyledPointVertexStorage,
  >(
    cx, "wide points buffer pool", 100, 1000000, data_source
  );

  let (cx, params) = cx.use_storage_buffer_with_host_backup::<WideStyledPointParameters>(
    "wide points buffer parameters and range info",
    128,
    u32::MAX,
  );

  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  range_change.update_storage_array_with_host(cx, params, 0);

  params.use_max_item_count_by_db_entity::<WideStyledPointsEntity>(cx);
  params.use_update(cx);

  let sm_to_wide_points_device =
    use_db_device_foreign_key::<SceneModelWideStyledPointsRenderPayload>(cx);

  cx.when_render(|| WideStyledPointsIndirectRenderer {
    model_access: global_database().read_foreign_key::<SceneModelWideStyledPointsRenderPayload>(),
    points: points_buffer,
    params: params.get_gpu_buffer(),
    sm_to_wide_points_device: sm_to_wide_points_device.unwrap(),
    params_host: params.buffer.make_read_holder(),
    used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
  })
}

pub struct WideStyledPointsIndirectRenderer {
  model_access: ForeignKeyReadView<SceneModelWideStyledPointsRenderPayload>,
  points: AbstractReadonlyStorageBuffer<[WideStyledPointVertexStorage]>,
  params: AbstractReadonlyStorageBuffer<[WideStyledPointParameters]>,
  sm_to_wide_points_device: AbstractReadonlyStorageBuffer<[u32]>,
  /// we keep the host metadata to support creating draw commands from host
  params_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<WideStyledPointParameters>>,
  used_in_midc_downgrade: bool,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct WideStyledPointParameters {
  pub range: Vec2<u32>,
  pub color: Vec3<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct WideStyledPointVertexStorage {
  pub position: Vec3<f32>,
  pub width: f32,
  pub style_id: u32,
}

impl WideStyledPointVertexStorage {
  fn u32_size() -> u32 {
    std::mem::size_of::<Self>() as u32 / 4
  }
}

impl IndirectModelRenderImpl for WideStyledPointsIndirectRenderer {
  fn hash_shader_group_key(
    &self,
    any_id: EntityHandle<SceneModelEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    self.model_access.get(any_id)?;
    Some(())
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn device_id_injector(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    self.model_access.get(any_idx)?;
    Some(Box::new(()))
  }

  fn shape_renderable_indirect(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    self.model_access.get(any_idx)?;
    Some(Box::new(WidePointsIndirectDrawComponent {
      points: self.points.clone(),
      params: self.params.clone(),
      sm_to_wide_points_device: self.sm_to_wide_points_device.clone(),
    }))
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    self.model_access.get(batch.impl_select_id)?;

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
    self.model_access.get(any_idx)?;

    let creator = WidePointsDrawCreator {
      params: self.params.clone(),
      params_host: self.params_host.clone(),
      sm_to_wide_points_device: self.sm_to_wide_points_device.clone(),
      sm_to_wide: self.model_access.clone(),
    };

    DrawCommandBuilder::NoneIndexed(Box::new(creator)).into()
  }

  fn material_renderable_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    self.model_access.get(any_idx)?;
    Some(Box::new(()))
  }
}

pub struct WidePointsIndirectDrawComponent {
  points: AbstractReadonlyStorageBuffer<[WideStyledPointVertexStorage]>,
  params: AbstractReadonlyStorageBuffer<[WideStyledPointParameters]>,
  sm_to_wide_points_device: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for WidePointsIndirectDrawComponent {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for WidePointsIndirectDrawComponent {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.sm_to_wide_points_device);
    ctx.binding.bind(&self.params);
    ctx.binding.bind(&self.points);
  }
}

only_vertex!(WidePointsWidthShader, f32);

impl GraphicsShaderProvider for WidePointsIndirectDrawComponent {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let sm_id = builder.query::<LogicalRenderEntityId>();
      let sm_to_wide_points_device = binding.bind_by(&self.sm_to_wide_points_device);
      let points_id = sm_to_wide_points_device.index(sm_id).load();

      let params = binding.bind_by(&self.params);
      let meta = params.index(points_id).load().expand();
      let points_range = meta.range;
      //   builder.register::<WideLineWidthShader>(points_range.width);

      let points = binding.bind_by(&self.points);

      let vertex_index = builder.query::<VertexIndex>();
      let instance_index = vertex_index / val(6);
      let vertex_index = vertex_index % val(6);

      let vertex = val(Vec2::new(0., 0.)).make_local_var();

      switch_by(vertex_index)
        .case(0, || vertex.store(Vec2::new(0., 0.)))
        .case(1, || vertex.store(Vec2::new(1., 1.)))
        .case(2, || vertex.store(Vec2::new(1., 0.)))
        .case(3, || vertex.store(Vec2::new(0., 0.)))
        .case(4, || vertex.store(Vec2::new(0., 1.)))
        .case(5, || vertex.store(Vec2::new(1., 1.)))
        .end_with_default(|| {});

      let vertex = vertex.load();
      builder.register::<GeometryPosition>(Node::from((vertex, val(0.))));
      builder.register::<GeometryUV>(vertex);

      let point = points
        .index(instance_index + points_range.x() / val(WideStyledPointVertexStorage::u32_size()))
        .load()
        .expand();

      builder.register::<WidePointPosition>(point.position);
      builder.register::<WidePointSize>(point.width);
      builder.set_vertex_out::<WidePointStyleId>(point.style_id);
      builder.register::<GeometryColorWithAlpha>((meta.color, val(1.)));

      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      let uv = builder.query::<GeometryUV>();
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>();
      let width = builder.query::<WidePointSize>();

      wide_line_vertex(
        builder.query::<WidePointPosition>(),
        builder.query::<GeometryPosition>(),
        builder.query::<ViewportRenderBufferSize>(),
        width,
        builder,
      );

      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<DefaultDisplay>(color_with_alpha);
    });

    builder.fragment(|builder, _| {
      let uv = builder.query::<FragmentUv>();
      // reject lighting
      builder.insert_type_tag::<UnlitMaterialTag>();

      let coord = uv * val(Vec2::new(2., 2.)) - val(Vec2::new(1., 1.));
      let style_id = builder.query::<WidePointStyleId>();
      let color = builder.query::<DefaultDisplay>();

      let (alpha, color_multiplier) = point_style_entry(coord, style_id);

      let final_color: Node<Vec4<f32>> = (color.xyz() * color_multiplier, alpha).into();

      builder.register::<DefaultDisplay>(final_color);

      builder.frag_output.iter_mut().for_each(|p| {
        if p.is_blendable() {
          p.states.blend = BlendState::ALPHA_BLENDING.into();
        }
      });
      if let Some(depth) = &mut builder.depth_stencil {
        depth.depth_write_enabled = false;
      }
    })
  }
}

#[derive(Clone)]
struct WidePointsDrawCreator {
  params: AbstractReadonlyStorageBuffer<[WideStyledPointParameters]>,
  sm_to_wide_points_device: AbstractReadonlyStorageBuffer<[u32]>,
  sm_to_wide: ForeignKeyReadView<SceneModelWideStyledPointsRenderPayload>,
  params_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<WideStyledPointParameters>>,
}

impl ShaderHashProvider for WidePointsDrawCreator {
  shader_hash_type_id! {}
}

impl NoneIndexedDrawCommandBuilder for WidePointsDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let model = self.sm_to_wide.get(id).unwrap();
    let param = self.params_host.get(model.alloc_index()).unwrap().range;
    let seg_count = param.y / WideStyledPointVertexStorage::u32_size();

    if param.x == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
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
    let sm_to_wide_points_device = cx.bind_by(&self.sm_to_wide_points_device);
    Box::new(DrawCmdBuilderInvocation {
      params,
      sm_to_wide_points_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.params);
    builder.bind(&self.sm_to_wide_points_device);
  }
}

struct DrawCmdBuilderInvocation {
  params: ShaderReadonlyPtrOf<[WideStyledPointParameters]>,
  sm_to_wide_points_device: ShaderReadonlyPtrOf<[u32]>,
}

impl NoneIndexedDrawCommandBuilderInvocation for DrawCmdBuilderInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndirectArgsStorage> {
    let point_id = self.sm_to_wide_points_device.index(draw_id).load();
    // the implementation of range allocate assure the count is zero if allocation failed
    let seg_count = self.params.index(point_id).load().expand().range.y()
      / val(WideStyledPointVertexStorage::u32_size());

    ENode::<DrawIndirectArgsStorage> {
      vertex_count: val(6) * seg_count,
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
