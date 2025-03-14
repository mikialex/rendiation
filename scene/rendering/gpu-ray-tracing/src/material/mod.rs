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

#[derive(Default)]
pub struct RtxSceneMaterialSource {
  material_ty: QueryToken,
  material_id: QueryToken,
  materials: Vec<BoxedQueryBasedGPUFeature<Box<dyn SceneMaterialSurfaceSupport>>>,
}

impl RtxSceneMaterialSource {
  pub fn register_resource(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
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

    let material_id = create_reactive_storage_buffer_container::<u32>(cx)
      .with_source(sm_to_mr)
      .with_source(sm_to_sg);

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

    let material_ty = create_reactive_storage_buffer_container::<u32>(cx).with_source(material_ty);
    self.material_id = qcx.register_multi_updater(material_id);
    self.material_ty = qcx.register_multi_updater(material_ty);
    for m in &mut self.materials {
      m.register(qcx, cx);
    }
  }
  pub fn deregister_resource(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.material_ty);
    qcx.deregister(&mut self.material_id);
    for m in &mut self.materials {
      m.deregister(qcx);
    }
  }
  pub fn create_impl(
    &self,
    cx: &mut QueryResultCtx,
    tex: &GPUTextureBindingSystem,
  ) -> SceneSurfaceSupport {
    let sm_to_material_type = cx
      .take_multi_updater_updated::<CommonStorageBufferImpl<u32>>(self.material_ty)
      .unwrap()
      .inner
      .gpu()
      .clone();

    let sm_to_material_id = cx
      .take_multi_updater_updated::<CommonStorageBufferImpl<u32>>(self.material_id)
      .unwrap()
      .inner
      .gpu()
      .clone();

    SceneSurfaceSupport {
      textures: tex.clone(),
      sm_to_material_type,
      sm_to_material_id,
      material_accessor: Arc::new(
        self
          .materials
          .iter()
          .map(|m| m.create_impl(cx))
          .collect::<Vec<_>>(),
      ),
    }
  }
}

impl RtxSceneMaterialSource {
  pub fn with_material_support(
    mut self,
    m: impl QueryBasedFeature<Box<dyn SceneMaterialSurfaceSupport>, Context = GPU> + 'static,
  ) -> Self {
    self.materials.push(Box::new(m));
    self
  }
}
