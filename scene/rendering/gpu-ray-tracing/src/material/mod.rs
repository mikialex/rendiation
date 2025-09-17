use rendiation_webgpu_hook_utils::*;

use crate::*;

mod mr;
mod sg;

/// for simplicity we not expect shader variant, so skip shader hashing
pub trait SceneMaterialSurfaceSupport {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn SceneMaterialSurfaceSupportInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait SceneMaterialSurfaceSupportInvocation {
  fn inject_material_info(
    &self,
    reg: &mut SemanticRegistry,
    material_id: Node<u32>,
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  );
}

pub fn use_rtx_scene_material(
  cx: &mut QueryGPUHookCx,
  materials: Option<Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>>,
  tex: Option<GPUTextureBindingSystem>,
) -> Option<SceneSurfaceSupport> {
  let (cx, material_id) = cx.use_storage_buffer("scene model ref material id", 128, u32::MAX);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  cx.use_dual_query::<StandardModelRefPbrMRMaterial>()
    .map(|q| q.filter_map(|id| id.map(|v| v.index())))
    .fanout(relation, cx)
    .into_delta_change()
    .update_storage_array(cx, material_id, 0);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  cx.use_dual_query::<StandardModelRefPbrSGMaterial>()
    .map(|q| q.filter_map(|id| id.map(|v| v.index())))
    .fanout(relation, cx)
    .into_delta_change()
    .update_storage_array(cx, material_id, 0);

  material_id.use_update(cx);
  material_id.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);

  let (cx, material_ty_gpu) =
    cx.use_storage_buffer("scene model ref material type id", 128, u32::MAX);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  let mr_material_ty = cx
    .use_dual_query::<StandardModelRefPbrMRMaterial>()
    .dual_query_filter_map(|v| v.map(|_| 0))
    .fanout(relation, cx);

  let relation = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();
  let sg_material_ty = cx
    .use_dual_query::<StandardModelRefPbrSGMaterial>()
    .dual_query_filter_map(|v| v.map(|_| 1))
    .fanout(relation, cx);

  let material_ty = mr_material_ty.dual_query_select(sg_material_ty);

  // todo, this query maybe overkill
  cx.use_dual_query_set::<SceneModelEntity>()
    .dual_query_union(material_ty, |(a, b)| a.map(|_| b.unwrap_or(u32::MAX)))
    .into_delta_change()
    .update_storage_array(cx, material_ty_gpu, 0);

  material_ty_gpu.use_update(cx);
  material_ty_gpu.use_max_item_count_by_db_entity::<SceneModelEntity>(cx);

  cx.when_render(|| SceneSurfaceSupport {
    textures: tex.unwrap().clone(),
    sm_to_material_type: material_ty_gpu.get_gpu_buffer(),
    sm_to_material_id: material_id.get_gpu_buffer(),
    material_accessor: materials.unwrap(),
  })
}
