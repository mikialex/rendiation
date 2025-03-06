use rendiation_webgpu_reactive_utils::{CommonStorageBufferImpl, ReactiveStorageBufferContainer};

use crate::*;

mod mr;

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
  material_ty: UpdateResultToken,
  material_id: UpdateResultToken,
  materials: Vec<Box<dyn RenderImplProvider<Box<dyn SceneMaterialSurfaceSupport>>>>,
}

impl RtxSceneMaterialSource {
  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let material_pbr_mr = global_watch()
      .watch::<StandardModelRefPbrMRMaterial>()
      .collective_filter_map(|id| id.map(|v| v.index()))
      .into_boxed();

    let sm_to_mr = material_pbr_mr
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>());

    let material_id = ReactiveStorageBufferContainer::<u32>::new(cx).with_source(sm_to_mr, 0);

    let material_ty_base = global_watch()
      .watch::<SceneModelStdModelRenderPayload>()
      .collective_map(|_| u32::MAX);

    let material_ty = global_watch()
      .watch::<StandardModelRefPbrMRMaterial>()
      .collective_map(|_| 0)
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>());

    let material_ty =
      material_ty_base.collective_union(material_ty, |(a, b)| a.map(|a| b.unwrap_or(a)));

    let material_ty = ReactiveStorageBufferContainer::<u32>::new(cx).with_source(material_ty, 0);
    self.material_id = source.register_multi_updater(material_id.inner);
    self.material_ty = source.register_multi_updater(material_ty.inner);
  }
  pub fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.material_ty);
    source.deregister(&mut self.material_id);
  }
  pub fn create_impl(
    &self,
    res: &mut QueryResultCtx,
    tex: &GPUTextureBindingSystem,
  ) -> SceneSurfaceSupport {
    let sm_to_material_type = res
      .take_multi_updater_updated::<CommonStorageBufferImpl<u32>>(self.material_ty)
      .unwrap()
      .inner
      .gpu()
      .clone();

    let sm_to_material_id = res
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
          .map(|m| m.create_impl(res))
          .collect::<Vec<_>>(),
      ),
    }
  }
}

impl RtxSceneMaterialSource {
  pub fn with_material_support(
    mut self,
    m: impl RenderImplProvider<Box<dyn SceneMaterialSurfaceSupport>> + 'static,
  ) -> Self {
    self.materials.push(Box::new(m));
    self
  }
}
