use std::sync::Arc;

use database::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod list_buffer;
use list_buffer::*;

mod extractor;
pub use extractor::{
  IncrementalDeviceSceneBatchExtractor, IncrementalDeviceSceneBatchExtractorShared,
};

pub fn use_incremental_device_scene_batch_extractor(
  cx: &mut QueryGPUHookCx,
  foreign_impl: GroupKeyForeignImpl,
) -> Option<IncrementalDeviceSceneBatchExtractorShared> {
  let sm_group_key = use_scene_model_group_key(cx, foreign_impl);

  let scene_id = cx
    .use_dual_query::<SceneModelBelongsToScene>()
    .dual_query_filter_map(|v| v);

  let group_key = sm_group_key.dual_query_zip(scene_id).dual_query_boxed();

  let visible_scene_models = use_global_node_net_visible(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx)
    .dual_query_filter_map(|v| v.then_some(()))
    .dual_query_boxed();

  let group_key = group_key
    .dual_query_filter_by_set(visible_scene_models)
    .dual_query_boxed();

  let (cx, extractor) =
    cx.use_plain_state_default_cloned::<IncrementalDeviceSceneBatchExtractorShared>();

  let extractor_ = extractor.clone();
  let gpu_updates = group_key
    .map_spawn_stage_in_thread_dual_query(cx, move |v| {
      let change = v.delta();
      Arc::new(extractor_.write().prepare_updates(change))
    })
    .use_assure_result(cx);

  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    extractor.write().do_updates(
      &gpu_updates.expect_resolve_stage(),
      &cx.storage_allocator,
      cx.gpu,
      encoder,
    );

    Some(extractor)
  } else {
    None
  }
}

#[derive(Default)]
pub struct GroupKeyForeignImpl {
  pub model: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>>>,
  pub mesh: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MeshGroupKey>>>,
  pub material: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MaterialGroupKey>>>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MeshGroupKey {
  Attribute {
    is_index: bool,
    topology: rendiation_scene_core::PrimitiveTopology,
  },
  ForeignHash(u64),
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

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
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
