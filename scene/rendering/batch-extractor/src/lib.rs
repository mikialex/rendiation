use database::*;
use fast_hash_collection::FastHashMap;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

pub fn use_incremental_device_scene_batch_extractor(
  cx: &mut QueryGPUHookCx,
  foreign_impl: GroupKeyForeignImpl,
) -> Option<IncrementalDeviceSceneBatchExtractor> {
  let sm_group_key = use_scene_model_group_key(cx, foreign_impl);

  let scene_id = cx.use_dual_query::<SceneModelBelongsToScene>();

  let group_key = sm_group_key.dual_query_zip(scene_id);

  let scene_model_net_visible =
    use_global_node_net_visible(cx).fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx);

  todo!()
}

struct PersistSceneModelListBuffer {
  buffer: PersistSceneModelListBufferWithLength,
  host: Vec<RawEntityHandle>,
}

#[derive(Clone)]
struct PersistSceneModelListBufferWithLength {
  buffer: AbstractReadonlyStorageBuffer<[u32]>,
}

impl DeviceParallelCompute<Node<u32>> for PersistSceneModelListBufferWithLength {
  fn execute_and_expose(
    &self,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    todo!()
  }

  fn result_size(&self) -> u32 {
    todo!()
  }
}
impl DeviceParallelComputeIO<u32> for PersistSceneModelListBufferWithLength {}

impl PersistSceneModelListBuffer {
  pub fn with_capacity(capacity: usize, alloc: &dyn AbstractStorageAllocator, gpu: &GPU) -> Self {
    let init_byte_size = (capacity + 1) * std::mem::size_of::<u32>();

    Self {
      buffer: PersistSceneModelListBufferWithLength {
        buffer: alloc.allocate_readonly(
          init_byte_size as u64,
          &gpu.device,
          Some("PersistSceneModelListBuffer"),
        ),
      },
      host: Default::default(),
    }
  }
}

pub struct IncrementalDeviceSceneBatchExtractor {
  contents: FastHashMap<
    EntityHandle<SceneEntity>,
    FastHashMap<SceneModelGroupKey, PersistSceneModelListBuffer>,
  >,
}

impl IncrementalDeviceSceneBatchExtractor {
  pub fn extract_scene_batch(
    &self,
    scene: EntityHandle<SceneEntity>,
    semantic: SceneContentKey,
  ) -> SceneModelRenderBatch {
    let contents = self.contents.get(&scene).unwrap();
    let sub_batches = if let Some(alpha_blend) = semantic.only_alpha_blend_objects {
      contents
        .iter()
        .filter(|(k, _)| k.require_alpha_blend() == alpha_blend)
        .map(|(_, v)| DeviceSceneModelRenderSubBatch {
          scene_models: Box::new(v.buffer.clone()),
          impl_select_id: unsafe { EntityHandle::from_raw(*v.host.first().unwrap()) },
        })
        .collect()
    } else {
      contents
        .values()
        .map(|v| DeviceSceneModelRenderSubBatch {
          scene_models: Box::new(v.buffer.clone()),
          impl_select_id: unsafe { EntityHandle::from_raw(*v.host.first().unwrap()) },
        })
        .collect()
    };
    let batches = DeviceSceneModelRenderBatch {
      sub_batches,
      stash_culler: None,
    };
    SceneModelRenderBatch::Device(batches)
  }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SceneModelGroupKey {
  Standard {
    material: MaterialGroupKey,
    mesh: MeshGroupKey,
  },
  ForeignHash {
    internal: u64,
    require_alpha_blend: bool,
  },
}

impl SceneModelGroupKey {
  pub fn require_alpha_blend(&self) -> bool {
    match self {
      SceneModelGroupKey::Standard { material, .. } => material.require_alpha_blend(),
      SceneModelGroupKey::ForeignHash {
        require_alpha_blend,
        ..
      } => *require_alpha_blend,
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub enum MeshGroupKey {
  Attribute {
    is_index: bool,
    topology: rendiation_scene_core::PrimitiveTopology,
  },
  ForeignHash(u64),
}

#[derive(Clone, PartialEq, Debug)]
pub enum MaterialGroupKey {
  Common {
    ty: u8,
    require_alpha_blend: bool,
  },
  ForeignHash {
    internal: u64,
    require_alpha_blend: bool,
  },
}

impl MaterialGroupKey {
  pub fn require_alpha_blend(&self) -> bool {
    match self {
      MaterialGroupKey::Common {
        require_alpha_blend,
        ..
      } => *require_alpha_blend,
      MaterialGroupKey::ForeignHash {
        require_alpha_blend,
        ..
      } => *require_alpha_blend,
    }
  }
}

#[derive(Default)]
pub struct GroupKeyForeignImpl {
  pub model: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>>>,
  pub mesh: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MeshGroupKey>>>,
  pub material: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MaterialGroupKey>>>,
}

fn use_scene_model_group_key(
  cx: &mut QueryGPUHookCx,
  foreign: GroupKeyForeignImpl,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>> {
  let material = use_indirect_material_indirect_group_key(cx);
  let material = if let Some(foreign) = foreign.material {
    material.dual_query_select(foreign).dual_query_boxed()
  } else {
    material
  };

  let mesh = attribute_mesh_group_key(cx);
  let mesh = if let Some(foreign) = foreign.mesh {
    mesh.dual_query_select(foreign).dual_query_boxed()
  } else {
    mesh
  };

  let sm_ref = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();

  let r = material
    .dual_query_zip(mesh)
    .dual_query_map(|(material, mesh)| SceneModelGroupKey::Standard { material, mesh })
    .fanout(sm_ref, cx)
    .dual_query_boxed();

  if let Some(foreign) = foreign.model {
    r.dual_query_select(foreign).dual_query_boxed()
  } else {
    r
  }
}

fn attribute_mesh_group_key(
  cx: &mut QueryGPUHookCx,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, MeshGroupKey>> {
  let is_index = cx
    .use_dual_query::<SceneBufferViewBufferId<AttributeIndexRef>>()
    .dual_query_map(|v| v.is_some());

  let topology = cx.use_dual_query::<AttributesMeshEntityTopology>();
  let model_ref = cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>();

  is_index
    .dual_query_zip(topology)
    .dual_query_map(|(is_index, topology)| MeshGroupKey::Attribute { is_index, topology })
    .fanout(model_ref, cx)
    .dual_query_boxed()
}

fn use_indirect_material_indirect_group_key(
  cx: &mut QueryGPUHookCx,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, MaterialGroupKey>> {
  let model_ref = cx.use_db_rev_ref_tri_view::<StandardModelRefPbrMRMaterial>();
  let m1 = cx
    .use_dual_query::<AlphaModeOf<PbrMRMaterialAlphaConfig>>()
    .dual_query_map(|v| to_common_material_key(v, 0))
    .fanout(model_ref, cx)
    .dual_query_boxed();

  let model_ref = cx.use_db_rev_ref_tri_view::<StandardModelRefPbrSGMaterial>();
  let m2 = cx
    .use_dual_query::<AlphaModeOf<PbrSGMaterialAlphaConfig>>()
    .dual_query_map(|v| to_common_material_key(v, 1))
    .fanout(model_ref, cx)
    .dual_query_boxed();

  let model_ref = cx.use_db_rev_ref_tri_view::<StandardModelRefUnlitMaterial>();
  let m3 = cx
    .use_dual_query::<AlphaModeOf<UnlitMaterialAlphaConfig>>()
    .dual_query_map(|v| to_common_material_key(v, 2))
    .fanout(model_ref, cx)
    .dual_query_boxed();

  m1.dual_query_select(m2)
    .dual_query_select(m3)
    .dual_query_boxed()
}

fn to_common_material_key(a: AlphaMode, ty: u8) -> MaterialGroupKey {
  MaterialGroupKey::Common {
    ty,
    require_alpha_blend: a == AlphaMode::Blend,
  }
}
