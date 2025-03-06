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
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  );
}

#[derive(Default)]
pub struct RtxSceneMaterialSource {
  material_ty: UpdateResultToken,
  materials: Vec<Box<dyn RenderImplProvider<Box<dyn SceneMaterialSurfaceSupport>>>>,
}

impl RtxSceneMaterialSource {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {}
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {}
  pub fn create_impl(
    &self,
    res: &mut QueryResultCtx,
    tex: &GPUTextureBindingSystem,
  ) -> SceneSurfaceSupport {
    todo!()
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
