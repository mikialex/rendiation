use crate::*;

impl RenderImplProvider<Box<dyn SceneMaterialSurfaceSupport>>
  for PbrMRMaterialDefaultIndirectRenderImplProvider
{
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    (self as &mut dyn RenderImplProvider<PbrMRMaterialDefaultIndirectRenderImpl>)
      .register_resource(source, cx);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    (self as &mut dyn RenderImplProvider<PbrMRMaterialDefaultIndirectRenderImpl>)
      .deregister_resource(source);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn SceneMaterialSurfaceSupport> {
    Box::new(
      (self as &dyn RenderImplProvider<PbrMRMaterialDefaultIndirectRenderImpl>).create_impl(res),
    )
  }
}

impl SceneMaterialSurfaceSupport for PbrMRMaterialDefaultIndirectRenderImpl {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn SceneMaterialSurfaceSupportInvocation> {
    Box::new(PbrMRMaterialRtxInvocation {
      storage: cx.bind_by(&self.storages),
      texture_storages: cx.bind_by(&self.tex_storages),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.storages);
    cx.bind(&self.storages);
  }
}

struct PbrMRMaterialRtxInvocation {
  pub storage: ShaderReadonlyPtrOf<[PhysicalMetallicRoughnessMaterialStorage]>,
  pub texture_storages:
    ShaderReadonlyPtrOf<[PhysicalMetallicRoughnessMaterialTextureHandlesStorage]>,
}

impl SceneMaterialSurfaceSupportInvocation for PbrMRMaterialRtxInvocation {
  fn inject_material_info(
    &self,
    reg: &mut SemanticRegistry,
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  ) {
    todo!()
  }
}
