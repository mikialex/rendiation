use rendiation_lighting_transport::*;

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
    id: Node<u32>,
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  ) {
    let storage = self.storage.index(id).load().expand();
    let tex_storage = self.texture_storages.index(id).load().expand();

    let mut alpha = storage.alpha;
    let mut base_color = storage.base_color;

    let base_color_alpha_tex = bind_and_sample(
      textures,
      reg,
      tex_storage.base_color_alpha_texture,
      uv,
      val(Vec4::one()),
    );
    alpha *= base_color_alpha_tex.w();
    base_color *= base_color_alpha_tex.xyz();

    let mut metallic = storage.metallic;
    let mut roughness = storage.roughness;

    let metallic_roughness_tex = bind_and_sample(
      textures,
      reg,
      tex_storage.metallic_roughness_texture,
      uv,
      val(Vec4::one()),
    );

    metallic *= metallic_roughness_tex.x();
    roughness *= metallic_roughness_tex.y();

    let mut emissive = storage.emissive;
    emissive *= bind_and_sample(
      textures,
      reg,
      tex_storage.emissive_texture,
      uv,
      val(Vec4::one()),
    )
    .xyz();

    reg.register_fragment_stage::<ColorChannel>(base_color);
    reg.register_fragment_stage::<EmissiveChannel>(emissive);
    reg.register_fragment_stage::<MetallicChannel>(metallic);
    reg.register_fragment_stage::<RoughnessChannel>(roughness * roughness);
  }
}
