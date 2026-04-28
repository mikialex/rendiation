use crate::*;

#[derive(Default)]
pub struct GroupKeyForeignImpl {
  /// sm id. -> sm key
  pub model: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>>>,
  /// std model id -> mesh key
  pub mesh: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MeshGroupKey>>>,
  /// std model id -> material key
  pub material: Option<UseResult<BoxedDynDualQuery<RawEntityHandle, MaterialGroupKey>>>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SceneModelGroupKey {
  Standard {
    material: MaterialGroupKey,
    mesh: MeshGroupKey,
    state_id: Option<InternedId<RasterizationStates>>,
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

pub fn use_scene_model_group_key(
  cx: &mut QueryGPUHookCx,
  foreign: GroupKeyForeignImpl,
) -> UseResult<BoxedDynDualQuery<RawEntityHandle, (SceneModelGroupKey, RawEntityHandle)>> {
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

  let state_id = cx.use_shared_dual_query(StateIntern);

  let r = material
    .dual_query_zip(mesh)
    .dual_query_union(state_id, |(a, s)| Some((a?, s)))
    .dual_query_map(
      |((material, mesh), state_id)| SceneModelGroupKey::Standard {
        material,
        mesh,
        state_id,
      },
    )
    .fanout(sm_ref, cx)
    .dual_query_boxed();

  let sm_group_key = if let Some(foreign) = foreign.model {
    r.dual_query_select(foreign).dual_query_boxed()
  } else {
    r
  };

  let scene_id = cx
    .use_dual_query::<SceneModelBelongsToScene>()
    .dual_query_filter_map(|v| v);

  let group_key = sm_group_key.dual_query_zip(scene_id).dual_query_boxed();

  let visible_scene_models = use_global_node_net_visible(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx)
    .dual_query_filter_map(|v| v.then_some(()))
    .dual_query_boxed();

  group_key
    .dual_query_filter_by_set(visible_scene_models)
    .dual_query_boxed()
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MeshGroupKey {
  Attribute {
    is_index: bool,
    topology: rendiation_scene_core::MeshPrimitiveTopology,
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
