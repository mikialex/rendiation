use std::sync::Arc;

use parking_lot::RwLock;
use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_webgpu_midc_downgrade::require_midc_downgrade;

use crate::*;

pub fn use_widen_line_indirect_renderer(
  cx: &mut QueryGPUHookCx,
  force_midc_downgrade: bool,
) -> Option<WideLineModelIndirectRenderer> {
  let init_item_count = 100;
  let max_item_count = 1000000;

  let item_byte_size = std::mem::size_of::<WideLineVertexStorage>() as u32;
  let (cx, line_seg_buffer) = cx.use_gpu_init(|gpu, alloc| {
    let buffer = alloc.allocate_readonly::<[WideLineVertexStorage]>(
      (item_byte_size * init_item_count) as u64,
      &gpu.device,
      Some("wide line segment buffer pool"),
    );

    let buffer = buffer.with_direct_resize(gpu);

    Arc::new(RwLock::new(buffer))
  });

  cx.if_inspect(|inspector| {
    let buffer_size = line_seg_buffer.read().gpu().byte_size();
    inspector.label_device_memory_usage("wide line segment buffer pool", buffer_size);
  });

  let (cx, allocator) =
    cx.use_sharable_plain_state(|| GrowableRangeAllocator::new(max_item_count, init_item_count));

  let gpu_buffer = line_seg_buffer.clone();

  // todo, improve code sharing with indirect attribute mesh
  let allocation_info = cx
    .use_dual_query::<WideLineMeshBuffer>()
    .map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
      let change = source_info.delta().into_change();
      let removed_and_changed_keys = change
        .iter_removed()
        .chain(change.iter_update_or_insert().map(|(k, _)| k));

      // todo, avoid resize
      let mut buffers_to_write = RangeAllocateBufferCollector::default();
      let mut sizes = Vec::new();

      for (k, buffer) in change.iter_update_or_insert() {
        let buffer = buffer.ptr.clone();

        // here we assume the buffer is correctly aligned
        let buffer: &[WideLineVertex] = cast_slice(&buffer);
        let buffer: Vec<_> = buffer
          .iter()
          .map(|v| WideLineVertexStorage {
            start: v.start,
            end: v.end,
            color: v.color,
            ..Default::default()
          })
          .collect();
        let buffer: &[u8] = cast_slice(&buffer);
        let len = buffer.len() as u32;
        buffers_to_write.collect_direct(k, buffer);
        sizes.push((k, len / item_byte_size));
      }

      let changes = allocator.write().update(removed_and_changed_keys, sizes);

      let buffers_to_write = buffers_to_write.prepare(&changes, item_byte_size);

      if let Some(new_size) = changes.resize_to {
        // here we do(request) resize at spawn stage to avoid resize again and again
        gpu_buffer.write().resize(new_size);
      }

      Arc::new(RangeAllocateBufferUpdates {
        buffers_to_write,
        allocation_changes: BatchAllocateResultShared(Arc::new(changes), item_byte_size / 4),
      })
    });

  let (allocation_info, allocation_info_) = allocation_info.fork();

  let allocation_info_ = allocation_info_.use_assure_result(cx);
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let mut gpu_buffer = line_seg_buffer.write();
    let gpu_buffer = gpu_buffer.abstract_gpu();
    allocation_info_
      .expect_resolve_stage()
      .write(cx.gpu, encoder, gpu_buffer);
  }

  let (cx, params) = cx.use_storage_buffer_with_host_backup::<WideLineParameters>(
    "wide line buffer parameters and range info",
    128,
    u32::MAX,
  );

  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());

  let offset = std::mem::offset_of!(WideLineParameters, data_range);
  range_change.update_storage_array_with_host(cx, params, offset);

  let width_change = cx.use_dual_query::<WideLineWidth>().into_delta_change();
  let offset = std::mem::offset_of!(WideLineParameters, width);
  width_change.update_storage_array_with_host(cx, params, offset);

  params.use_max_item_count_by_db_entity::<WideLineModelEntity>(cx);
  params.use_update(cx);

  let (cx, sm_to_wide_line_device) =
    cx.use_storage_buffer::<u32>("scene_model to wide line mapping", 128, u32::MAX);

  cx.use_dual_query::<StandardModelRefAttributesMeshEntity>()
    .map_raw_handle_or_u32_max_changes()
    .update_storage_array(cx, sm_to_wide_line_device, 0);

  sm_to_wide_line_device.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);
  sm_to_wide_line_device.use_update(cx);

  cx.when_render(|| WideLineModelIndirectRenderer {
    model_access: global_database().read_foreign_key::<SceneModelWideLineRenderPayload>(),
    segments: line_seg_buffer.read().gpu().clone(),
    params: params.get_gpu_buffer(),
    sm_to_wide_line_device: sm_to_wide_line_device.get_gpu_buffer(),
    params_host: params.buffer.make_read_holder(),
    used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
  })
}

pub struct WideLineModelIndirectRenderer {
  model_access: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  segments: AbstractReadonlyStorageBuffer<[WideLineVertexStorage]>,
  params: AbstractReadonlyStorageBuffer<[WideLineParameters]>,
  sm_to_wide_line_device: AbstractReadonlyStorageBuffer<[u32]>,
  /// we keep the host metadata to support creating draw commands from host
  params_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<WideLineParameters>>,
  used_in_midc_downgrade: bool,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct WideLineParameters {
  pub data_range: Vec2<u32>,
  pub width: f32,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct WideLineVertexStorage {
  pub start: Vec3<f32>,
  pub end: Vec3<f32>,
  pub color: Vec4<f32>,
}

impl WideLineVertexStorage {
  fn u32_size() -> u32 {
    std::mem::size_of::<Self>() as u32 / 4
  }
}

impl IndirectModelRenderImpl for WideLineModelIndirectRenderer {
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
    Some(Box::new(WideLineIndirectDrawComponent {
      segments: self.segments.clone(),
      params: self.params.clone(),
      sm_to_wide_line_device: self.sm_to_wide_line_device.clone(),
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

    let creator = WideLineDrawCreator {
      params: self.params.clone(),
      params_host: self.params_host.clone(),
      sm_to_wide_line_device: self.sm_to_wide_line_device.clone(),
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

pub struct WideLineIndirectDrawComponent {
  segments: AbstractReadonlyStorageBuffer<[WideLineVertexStorage]>,
  params: AbstractReadonlyStorageBuffer<[WideLineParameters]>,
  sm_to_wide_line_device: AbstractReadonlyStorageBuffer<[u32]>,
}

impl ShaderHashProvider for WideLineIndirectDrawComponent {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for WideLineIndirectDrawComponent {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.sm_to_wide_line_device);
    ctx.binding.bind(&self.params);
    ctx.binding.bind(&self.segments);
  }
}

only_vertex!(WideLineWidthShader, f32);

impl GraphicsShaderProvider for WideLineIndirectDrawComponent {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let sm_id = builder.query::<LogicalRenderEntityId>();
      let sm_to_wide_line_device = binding.bind_by(&self.sm_to_wide_line_device);
      let line_id = sm_to_wide_line_device.index(sm_id).load();

      let params = binding.bind_by(&self.params);
      let line_param = params.index(line_id).load().expand();
      builder.register::<WideLineWidthShader>(line_param.width);

      let segments = binding.bind_by(&self.segments);

      let vertex_index = builder.query::<VertexIndex>();
      let instance_index = vertex_index / val(18);
      let vertex_index = vertex_index % val(18);

      let v1 = (val(1) + vertex_index) % val(6); // index in quad
      let v2 = (val(2) + vertex_index) % val(6); // index in quad

      let dy = (vertex_index / val(6)).into_f32();
      let x = v1.less_equal_than(val(2)).select(val(-1.), val(1.));
      let y = v2.less_equal_than(val(2)).select(val(2.), val(1.));

      let y = y - dy;
      let y = y.less_equal_than(0.).select(y - val(1.), y);

      let vertex: Node<Vec2<f32>> = (x, y).into();
      builder.register::<GeometryPosition>(Node::from((vertex, val(0.))));
      builder.register::<GeometryUV>(vertex);

      let seg = segments
        .index(
          instance_index
            + line_param.data_range.x()
              / val(std::mem::size_of::<WideLineVertexStorage>() as u32 / 4),
        )
        .load()
        .expand();

      builder.register::<WideLineStart>(seg.start);
      builder.register::<WideLineEnd>(seg.end);
      builder.register::<GeometryColorWithAlpha>(seg.color);

      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      let uv = builder.query::<GeometryUV>();
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>();
      let width = builder.query::<WideLineWidthShader>();

      wide_line_vertex(
        builder.query::<WideLineStart>(),
        builder.query::<WideLineEnd>(),
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
      builder.insert_type_tag::<UnlitMaterialTag>();
      if_by(discard_round_corner_fn(uv), || {
        builder.discard();
      });
    })
  }
}

#[derive(Clone)]
struct WideLineDrawCreator {
  params: AbstractReadonlyStorageBuffer<[WideLineParameters]>,
  sm_to_wide_line_device: AbstractReadonlyStorageBuffer<[u32]>,
  sm_to_wide: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  params_host: LockReadGuardHolder<SparseStorageBufferWithHostRaw<WideLineParameters>>,
}

impl ShaderHashProvider for WideLineDrawCreator {
  shader_hash_type_id! {}
}

impl NoneIndexedDrawCommandBuilder for WideLineDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let model = self.sm_to_wide.get(id).unwrap();
    let param = self.params_host.get(model.alloc_index()).unwrap();
    let seg_count = param.data_range.y / WideLineVertexStorage::u32_size();

    if param.data_range.x == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
      return None;
    }

    DrawCommand::Array {
      instances: 0..1,
      vertices: 0..18 * seg_count,
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let params = cx.bind_by(&self.params);
    let sm_to_wide_line_device = cx.bind_by(&self.sm_to_wide_line_device);
    Box::new(DrawCmdBuilderInvocation {
      params,
      sm_to_wide_line_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.params);
    builder.bind(&self.sm_to_wide_line_device);
  }
}

struct DrawCmdBuilderInvocation {
  params: ShaderReadonlyPtrOf<[WideLineParameters]>,
  sm_to_wide_line_device: ShaderReadonlyPtrOf<[u32]>,
}

impl NoneIndexedDrawCommandBuilderInvocation for DrawCmdBuilderInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // aka sm id
  ) -> Node<DrawIndirectArgsStorage> {
    let line_id = self.sm_to_wide_line_device.index(draw_id).load();
    // the implementation of range allocate assure the count is zero if allocation failed
    let seg_count =
      self.params.index(line_id).data_range().load().y() / val(WideLineVertexStorage::u32_size());

    ENode::<DrawIndirectArgsStorage> {
      vertex_count: val(18) * seg_count,
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: val(0),
    }
    .construct()
  }
}
