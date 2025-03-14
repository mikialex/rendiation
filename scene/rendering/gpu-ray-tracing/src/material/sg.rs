use rendiation_lighting_transport::*;

use crate::*;

impl QueryBasedFeature<Box<dyn SceneMaterialSurfaceSupport>>
  for PbrSGMaterialDefaultIndirectRenderImplProvider
{
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    (self as &mut dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .register(qcx, cx);
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    (self as &mut dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
      .deregister(qcx);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn SceneMaterialSurfaceSupport> {
    Box::new(
      (self as &dyn QueryBasedFeature<PbrSGMaterialDefaultIndirectRenderImpl, Context = GPU>)
        .create_impl(cx),
    )
  }
}

impl SceneMaterialSurfaceSupport for PbrSGMaterialDefaultIndirectRenderImpl {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn SceneMaterialSurfaceSupportInvocation> {
    Box::new(PbrSGMaterialRtxInvocation {
      storage: cx.bind_by(&self.storages),
      texture_storages: cx.bind_by(&self.tex_storages),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.storages);
    cx.bind(&self.tex_storages);
  }
}

struct PbrSGMaterialRtxInvocation {
  pub storage: ShaderReadonlyPtrOf<[PhysicalSpecularGlossinessMaterialStorage]>,
  pub texture_storages:
    ShaderReadonlyPtrOf<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
}

impl SceneMaterialSurfaceSupportInvocation for PbrSGMaterialRtxInvocation {
  fn inject_material_info(
    &self,
    reg: &mut SemanticRegistry,
    id: Node<u32>,
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  ) {
    let storage = self.storage.index(id).load().expand();
    let tex_storage = self.texture_storages.index(id).load().expand();

    let mut alpha = storage.alpha;
    let mut base_color = storage.albedo;

    let albedo = indirect_sample(
      textures,
      reg,
      tex_storage.albedo_texture,
      uv,
      val(Vec4::one()),
    );
    alpha *= albedo.w();
    base_color *= albedo.xyz();

    let mut specular = storage.specular;
    let specular_glossiness = indirect_sample(
      textures,
      reg,
      tex_storage.specular_glossiness_texture,
      uv,
      val(Vec4::one()),
    );
    specular *= specular_glossiness.xyz();

    let glossiness = storage.glossiness * specular_glossiness.w();

    let mut emissive = storage.emissive;
    emissive *= indirect_sample(
      textures,
      reg,
      tex_storage.emissive_texture,
      uv,
      val(Vec4::one()),
    )
    .xyz();

    reg.register_fragment_stage::<ColorChannel>(base_color);
    reg.register_fragment_stage::<SpecularChannel>(specular);
    reg.register_fragment_stage::<EmissiveChannel>(emissive);
    reg.register_fragment_stage::<GlossinessChannel>(glossiness);
  }
}
