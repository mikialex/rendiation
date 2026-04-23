use crate::*;

pub fn use_view_dependent_transform_indirect_gpu(
  cx: &mut QueryGPUHookCx,
  changes: UseResult<BoxedDynDualQuery<ViewSceneModelKey, Mat4<f64>>>,
  internal_impl: Option<IndirectNodeRenderer>,
  control: CurrentViewControl,
) -> Option<Box<dyn IndirectNodeRenderImpl>> {
  let changes = changes.use_assure_result(cx);

  let overrides = cx.use_shared_hash_map::<ViewKey, PerViewGPUResource>("OverrideNodeIndirectGPU");

  // todo, move updates collect to thread pool
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let query = changes.expect_resolve_stage();

    let mut overrides_ = overrides.write();
    let overrides = &mut overrides_;
    let changes = query.delta.into_change();

    for (view_id, sm) in changes.iter_removed() {
      let view_data = overrides
        .entry(view_id)
        .or_insert_with(|| PerViewGPUResource::new(cx.gpu, cx.storage_allocator.as_ref()));
      view_data.remove(sm);
    }

    for ((view_id, sm), mat) in changes.iter_update_or_insert() {
      let view_data = overrides
        .entry(view_id)
        .or_insert_with(|| PerViewGPUResource::new(cx.gpu, cx.storage_allocator.as_ref()));
      view_data.update(sm, mat);
    }

    let size_require = database::global_database()
      .access_table_dyn(SceneModelEntity::entity_id(), |table| {
        table.entity_capacity()
      });

    for o in overrides.values_mut() {
      o.notify_max_sm_size(size_require);
    }

    overrides.retain(|_, o| o.flush(cx.gpu, encoder));
  }

  internal_impl.map(|internal| {
    Box::new(OverrideNodeIndirectGPU {
      internal,
      overrides: overrides.make_read_holder(),
      current_view: control,
    }) as Box<_>
  })
}

pub struct OverrideNodeIndirectGPU {
  internal: IndirectNodeRenderer,
  overrides: LockReadGuardHolder<FastHashMap<ViewKey, PerViewGPUResource>>,
  current_view: CurrentViewControl,
}

type GPUBufferImpl<T> = CustomGrowBehaviorMaintainer<
  BufferWidthDefaultValue<
    GPUStorageDirectQueueUpdate<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>,
  >,
>;

struct PerViewGPUResource {
  // todo, use SparseStorageBufferRaw
  overrides: SparseUpdateStorageBuffer<NodeStorage>,
  slab: Slab<()>,
  mapping: FastHashMap<RawEntityHandle, usize>,
  mapping_change: FastHashMap<RawEntityHandle, u32>,
  index_remap: GPUBufferImpl<u32>,
  overrides_updates: Option<SparseBufferWritesSource>,
}

impl PerViewGPUResource {
  pub fn new(gpu: &GPU, alloc: &dyn AbstractStorageAllocator) -> Self {
    let buffer = alloc.allocate_readonly(
      make_init_size::<u32>(128),
      &gpu.device,
      Some("PerViewGPUResource index_remap"),
    );

    let index_remap = buffer
      .with_direct_resize(gpu)
      .with_queue_direct_update(&gpu.queue)
      .with_default_value_with_init_write(u32::MAX)
      .with_default_grow_behavior(u32::MAX);

    Self {
      // todo, shrink this buffer
      overrides: SparseUpdateStorageBuffer::new(
        "PerViewGPUResource overrides",
        128,
        u32::MAX,
        alloc,
        gpu,
      ),
      index_remap,
      overrides_updates: None,
      mapping_change: Default::default(),
      slab: Default::default(),
      mapping: Default::default(),
    }
  }

  pub fn notify_max_sm_size(&mut self, max_sm_size: usize) {
    self.index_remap.resize(max_sm_size as u32);
    self
      .overrides
      .buffer
      .check_resize(self.slab.capacity() as u32);
  }

  pub fn update(&mut self, sm: RawEntityHandle, mat: Mat4<f64>) {
    if let Some(old_index) = self.mapping.remove(&sm) {
      self.slab.remove(old_index);
    }
    let new_index = self.slab.insert(());
    self.mapping.insert(sm, new_index);

    self.mapping_change.insert(sm, new_index as u32);

    // we are not cache change for this, because the data is not overlap(guaranteed by outside query)
    let node = NodeStorage::from_world_mat(mat);
    let data_writer = self
      .overrides_updates
      .get_or_insert_with(|| SparseBufferWritesSource::with_capacity(0, 0));
    let sm_data_byte_offset = new_index as u64 * std::mem::size_of::<NodeStorage>() as u64;
    data_writer.collect_write(bytes_of(&node), sm_data_byte_offset);
  }

  pub fn remove(&mut self, sm: RawEntityHandle) {
    let old_index = self.mapping.remove(&sm).unwrap();
    self.slab.remove(old_index);
    self.mapping_change.insert(sm, u32::MAX);
  }

  // return if should exist
  pub fn flush(&mut self, gpu: &GPU, encoder: &mut GPUCommandEncoder) -> bool {
    if let Some(updates) = self.overrides_updates.take() {
      updates.write_abstract(gpu, encoder, &self.overrides.get_gpu_buffer());
    }
    if self.mapping_change.len() > 0 {
      let mut updates = SparseBufferWritesSource::with_capacity(0, 0); // todo, capacity
      for (k, v) in self.mapping_change.drain() {
        let sm_index_byte_offset = k.index() as u64 * std::mem::size_of::<u32>() as u64;
        updates.collect_write(bytes_of(&v), sm_index_byte_offset);
      }
      updates.write_abstract(gpu, encoder, self.index_remap.gpu());
    }
    self.mapping.len() > 0
  }
}

impl IndirectNodeRenderImpl for OverrideNodeIndirectGPU {
  fn make_component_indirect(
    &self,
    _any_idx: EntityHandle<SceneNodeEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let overrides = if let Some(current_view) = self.current_view.get() {
      // the override view can not be found, if there is no a single view dep object in scene,
      // so we must not early return here
      self.overrides.get(&current_view)
    } else {
      None
    };
    Some(Box::new(NodeGPUStorageWithOverride {
      base: &self.internal.0,
      overrides,
    }))
  }

  fn hash_shader_group_key(
    &self,
    _any_id: EntityHandle<SceneNodeEntity>,
    _hasher: &mut PipelineHasher,
  ) -> Option<()> {
    Some(())
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }
}

pub struct NodeGPUStorageWithOverride<'a> {
  base: &'a AbstractReadonlyStorageBuffer<[NodeStorage]>,
  // this is optional, as in some case(shadow pass), it not exist any override data.
  overrides: Option<&'a PerViewGPUResource>,
}

impl<'a> ShaderHashProvider for NodeGPUStorageWithOverride<'a> {
  shader_hash_type_id! {NodeGPUStorageWithOverride<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.overrides.is_none().hash(hasher);
  }
}

impl GraphicsShaderProvider for NodeGPUStorageWithOverride<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let nodes = binding.bind_by(self.base);

      let current_node_id = builder.query::<IndirectSceneNodeId>();
      let sm_id = builder.query::<LogicalRenderEntityId>();

      let node = if let Some(overrides) = &self.overrides {
        let remap = binding.bind_by(overrides.index_remap.gpu());
        let overrides = binding.bind_by(&overrides.overrides.get_gpu_buffer());

        let remap_index = remap.index(sm_id).load();

        let node = zeroed_val::<NodeStorage>().make_local_var();

        if_by(remap_index.not_equals(val(u32::MAX)), || {
          node.store(overrides.index(remap_index).load());
        })
        .else_by(|| {
          node.store(nodes.index(current_node_id).load());
        });

        node.load().expand()
      } else {
        nodes.index(current_node_id).load().expand()
      };

      builder.register::<WorldNoneTranslationMatrix>(node.world_matrix_none_translation);
      builder.register::<WorldPositionHP>(hpt_storage_to_hpt(node.world_position_hp));
      builder.register::<WorldNormalMatrix>(node.normal_matrix);

      // the RenderVertexPosition requires camera, so here we only process normal part
      if let Some(normal) = builder.try_query::<GeometryNormal>() {
        builder.register::<VertexRenderNormal>(node.normal_matrix * normal);
      }
    })
  }
}

impl ShaderPassBuilder for NodeGPUStorageWithOverride<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.base);
    if let Some(overrides) = &self.overrides {
      ctx.binding.bind(overrides.index_remap.gpu());
      ctx.binding.bind(&overrides.overrides.get_gpu_buffer());
    }
  }
}
