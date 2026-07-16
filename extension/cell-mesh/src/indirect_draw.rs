use std::{any::TypeId, hash::Hash, sync::Arc};

use fast_hash_collection::fast_hash_scope;
use parking_lot::RwLock;
use rendiation_device_parallel_compute::FrameCtxParallelComputeExt;
use rendiation_scene_batch_extractor::MeshGroupKey;
use rendiation_webgpu_midc_downgrade::require_midc_downgrade;

use crate::*;

/// std model id -> mesh key
pub fn use_cell_mesh_group_key(
  cx: &mut impl DBHookCxLike,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, MeshGroupKey>> {
  cx.use_dual_query_set::<CellMeshEntity>()
    .dual_query_map(|_| {
      let hash = fast_hash_scope(|hasher| TypeId::of::<CellMeshEntity>().hash(hasher));
      MeshGroupKey::ForeignHash(hash)
    })
    .fanout(
      cx.use_db_rev_ref_tri_view::<StandardModelCellMeshPayload>(),
      cx,
    )
    .dual_query_boxed()
}

pub fn use_cell_mesh_renderer(
  cx: &mut QueryGPUHookCx,
  force_midc_downgrade: bool,
) -> Option<Box<dyn IndirectModelShapeRenderImpl>> {
  let data_source = cx
    .use_dual_query::<CellMeshUnitsBuffer>()
    .map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
      source_info.delta().into_change().collective_map(|buffer| {
        let new_buffer = buffer
          .iter()
          .map(|v| CellMeshUnitDataStorage {
            p1: v.p1,
            p2: v.p2,
            p3: v.p3,
            p4: v.p4,
            center: v.center,
            front_face_color: v.front_face_color,
            back_face_color: v.back_face_color,
            ..Default::default()
          })
          .collect::<Vec<_>>();
        ExternalRefPtr::new(new_buffer)
      })
    });

  let (units, allocation_info) = use_range_allocated_device_buffers::<CellMeshUnitDataStorage>(
    cx,
    "cell mesh unit buffer pool",
    100,
    u32::MAX,
    data_source,
  );

  let (cx, params) = cx.use_storage_buffer_with_host_backup::<CellMeshParameters>(
    "cell mesh parameters and range info",
    128,
    u32::MAX,
  );

  let range_change =
    allocation_info.map(|allocation_info| allocation_info.allocation_changes.clone());
  let offset = std::mem::offset_of!(CellMeshParameters, data_range);
  range_change.update_storage_array_with_host(cx, params, offset);

  let change = cx
    .use_dual_query::<CellMeshShrinkRatio>()
    .into_delta_change();
  let offset = std::mem::offset_of!(CellMeshParameters, shrink_ratio);
  change.update_storage_array_with_host(cx, params, offset);

  params.use_max_item_count_by_db_entity::<CellMeshEntity>(cx);
  params.use_update(cx);

  let params_host = params.buffer.clone();

  let std_model_to_cell_mesh_device = use_db_device_foreign_key::<StandardModelCellMeshPayload>(cx);

  cx.when_render(|| {
    Box::new(CellMeshRenderer {
      std_model_to_cell_mesh_id: read_global_db_foreign_key(),
      used_in_midc_downgrade: require_midc_downgrade(&cx.gpu.info, force_midc_downgrade),
      units,
      params: params.get_gpu_buffer(),
      params_host,
      std_model_to_cell_mesh_device: std_model_to_cell_mesh_device.unwrap(),
    }) as Box<dyn IndirectModelShapeRenderImpl>
  })
}

pub struct CellMeshRenderer {
  std_model_to_cell_mesh_id: ForeignKeyReadView<StandardModelCellMeshPayload>,
  used_in_midc_downgrade: bool,
  units: AbstractReadonlyStorageBuffer<[CellMeshUnitDataStorage]>,
  params: AbstractReadonlyStorageBuffer<[CellMeshParameters]>,
  params_host: Arc<RwLock<SparseStorageBufferWithHostRaw<CellMeshParameters>>>,
  std_model_to_cell_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
}

impl IndirectModelShapeRenderImpl for CellMeshRenderer {
  fn make_component_indirect(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Box<dyn RenderComponent + '_>> {
    let _ = self.std_model_to_cell_mesh_id.get(any_idx)?;

    Some(Box::new(CellMeshShape {
      units: self.units.clone(),
      params: self.params.clone(),
      std_model_to_cell_mesh_device: self.std_model_to_cell_mesh_device.clone(),
    }))
  }

  fn get_index_storage_buffer(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Option<AbstractReadonlyStorageBuffer<[u32]>>> {
    let _ = self.std_model_to_cell_mesh_id.get(any_idx)?;
    Some(None)
  }

  fn hash_shader_group_key(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
    _: &mut PipelineHasher,
  ) -> Option<()> {
    let _ = self.std_model_to_cell_mesh_id.get(any_idx)?;
    Some(())
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn generate_indirect_draw_provider(
    &self,
    batch: &DeviceSceneModelRenderSubBatch,
    any_idx: EntityHandle<StandardModelEntity>,
    ctx: &mut FrameCtx,
  ) -> Option<Box<dyn IndirectDrawProvider>> {
    let _ = self.std_model_to_cell_mesh_id.get(any_idx)?;

    let draw_command_builder = self.make_draw_command_builder(any_idx).unwrap();

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
    any_idx: EntityHandle<StandardModelEntity>,
  ) -> Option<DrawCommandBuilder> {
    let _ = self.std_model_to_cell_mesh_id.get(any_idx)?;

    let creator = CellMeshDrawCreator {
      params: self.params.clone(),
      params_host: self.params_host.clone(),
      std_model_to_cell_mesh_device: self.std_model_to_cell_mesh_device.clone(),
      std_model_to_cell_mesh_id: self.std_model_to_cell_mesh_id.clone(),
      sm_to_std_model: read_global_db_foreign_key(),
    };

    DrawCommandBuilder::NoneIndexed(Box::new(creator)).into()
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Copy, Clone, ShaderStruct, Default)]
pub struct CellMeshUnitDataStorage {
  // the position of the four vertices
  pub p1: Vec3<f32>,
  pub p2: Vec3<f32>,
  pub p3: Vec3<f32>,
  pub p4: Vec3<f32>,
  // the shrink center of this unit
  pub center: Vec3<f32>,

  pub front_face_color: Vec3<f32>,
  pub back_face_color: Vec3<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
struct CellMeshParameters {
  pub data_range: Vec2<u32>,
  pub shrink_ratio: f32,
}

pub struct CellMeshShape {
  units: AbstractReadonlyStorageBuffer<[CellMeshUnitDataStorage]>,
  params: AbstractReadonlyStorageBuffer<[CellMeshParameters]>,
  std_model_to_cell_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
}

impl GraphicsShaderProvider for CellMeshShape {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      let std_id = builder.query::<IndirectStdModelId>();

      let std_model_to_cell_mesh_device = binding.bind_by(&self.std_model_to_cell_mesh_device);
      let cell_mesh_id = std_model_to_cell_mesh_device.index(std_id).load();

      let vertex_index = builder.query::<VertexIndex>();
      let unit_index = vertex_index / val(6); // draw two triangle for each unit
      let unit_internal_index = vertex_index % val(6);

      let params = binding.bind_by(&self.params);
      let mesh_meta = params.index(cell_mesh_id).load().expand();
      let mesh_unit_start = mesh_meta.data_range.x();

      let units = binding.bind_by(&self.units);
      let unit = units.index(mesh_unit_start + unit_index);

      let position = zeroed_val::<Vec3<f32>>().make_local_var();
      switch_by(unit_internal_index)
        .case(0, || position.store(unit.p1().load()))
        .case(1, || position.store(unit.p4().load()))
        .case(2, || position.store(unit.p3().load()))
        .case(3, || position.store(unit.p1().load()))
        .case(4, || position.store(unit.p3().load()))
        .case(5, || position.store(unit.p2().load()))
        .end_with_default(|| {});
      let position = position.load();

      let center = unit.center().load();
      let shrink_ratio = mesh_meta.shrink_ratio;
      let position = shrink_ratio.mix(center, position);

      builder.register::<GeometryPosition>(position);
      // todo, use triangle stripe
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
    })
  }
}

impl ShaderPassBuilder for CellMeshShape {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.std_model_to_cell_mesh_device);
    ctx.binding.bind(&self.params);
    ctx.binding.bind(&self.units);
  }
}

impl ShaderHashProvider for CellMeshShape {
  shader_hash_type_id! {}
}

#[derive(Clone)]
struct CellMeshDrawCreator {
  params: AbstractReadonlyStorageBuffer<[CellMeshParameters]>,
  params_host: Arc<RwLock<SparseStorageBufferWithHostRaw<CellMeshParameters>>>,
  std_model_to_cell_mesh_device: AbstractReadonlyStorageBuffer<[u32]>,
  std_model_to_cell_mesh_id: ForeignKeyReadView<StandardModelCellMeshPayload>,
  sm_to_std_model: ForeignKeyReadView<SceneModelStdModelRenderPayload>,
}

impl ShaderHashProvider for CellMeshDrawCreator {
  shader_hash_type_id! {}
}

impl NoneIndexedDrawCommandBuilder for CellMeshDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    let std_model_id = self.sm_to_std_model.get(id)?;
    let cell_mesh_id = self.std_model_to_cell_mesh_id.get(std_model_id)?;
    let params = self.params_host.read();
    let param = params.get(cell_mesh_id.alloc_index())?;
    let unit_count = param.data_range.y;

    if param.data_range.x == DEVICE_RANGE_ALLOCATE_FAIL_MARKER {
      return None;
    }

    // two triangles per unit, six vertices total
    DrawCommand::Array {
      instances: 0..1,
      vertices: 0..(6 * unit_count),
    }
    .into()
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let params = cx.bind_by(&self.params);
    let std_model_to_cell_mesh_device = cx.bind_by(&self.std_model_to_cell_mesh_device);
    Box::new(CellMeshDrawCmdInvocation {
      params,
      std_model_to_cell_mesh_device,
    })
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.params);
    builder.bind(&self.std_model_to_cell_mesh_device);
  }
}

struct CellMeshDrawCmdInvocation {
  params: ShaderReadonlyPtrOf<[CellMeshParameters]>,
  std_model_to_cell_mesh_device: ShaderReadonlyPtrOf<[u32]>,
}

impl NoneIndexedDrawCommandBuilderInvocation for CellMeshDrawCmdInvocation {
  fn generate_draw_command(
    &self,
    draw_id: Node<u32>, // standard model id
  ) -> Node<DrawIndirectArgsStorage> {
    let cell_mesh_id = self.std_model_to_cell_mesh_device.index(draw_id).load();
    // range allocate assures the count is zero if allocation failed
    let unit_count = self.params.index(cell_mesh_id).data_range().load().y();

    // two triangles per unit, six vertices total
    ENode::<DrawIndirectArgsStorage> {
      vertex_count: val(6) * unit_count,
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }
    .construct()
  }
}
