use rendiation_webgpu_reactive_utils::*;

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
  let material_id = cx.use_storage_buffer(|cx| {
    let material_pbr_mr = global_watch()
      .watch::<StandardModelRefPbrMRMaterial>()
      .collective_filter_map(|id| id.map(|v| v.index()))
      .into_boxed();

    let sm_to_mr = material_pbr_mr
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
      .into_query_update_storage(0);

    let material_pbr_sg = global_watch()
      .watch::<StandardModelRefPbrSGMaterial>()
      .collective_filter_map(|id| id.map(|v| v.index()))
      .into_boxed();

    let sm_to_sg = material_pbr_sg
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
      .into_query_update_storage(0);

    create_reactive_storage_buffer_container::<u32>(128, u32::MAX, cx)
      .with_source(sm_to_mr)
      .with_source(sm_to_sg)
  });

  let material_ty = cx.use_storage_buffer(|cx| {
    let material_ty_base = global_watch().watch_entity_set::<SceneModelEntity>();

    let mr_material_ty = global_watch()
      .watch::<StandardModelRefPbrMRMaterial>()
      .collective_filter_map(|v| v.map(|_| 0))
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>());

    let sg_material_ty = global_watch()
      .watch::<StandardModelRefPbrSGMaterial>()
      .collective_filter_map(|v| v.map(|_| 1))
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>());

    let material_ty = mr_material_ty.collective_select(sg_material_ty);

    let material_ty = material_ty_base
      .collective_union(material_ty, |(a, b)| a.map(|_| b.unwrap_or(u32::MAX)))
      .into_query_update_storage(0);

    create_reactive_storage_buffer_container::<u32>(128, u32::MAX, cx).with_source(material_ty)
  });

  cx.when_create_impl(|| SceneSurfaceSupport {
    textures: tex.unwrap().clone(),
    sm_to_material_type: material_ty.unwrap(),
    sm_to_material_id: material_id.unwrap(),
    material_accessor: materials.unwrap(),
  })
}
