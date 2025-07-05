use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_lighting_transport::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod group;
pub use group::*;

mod array;
pub use array::*;

pub trait LightingComputeComponent: ShaderHashProvider {
  fn build_light_compute_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
    scene_id: Node<u32>,
  ) -> Box<dyn LightingComputeInvocation>;
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx);
}

pub trait GeometryCtxProvider: ShaderPassBuilder + ShaderHashProvider {
  /// the result node should be lived in fragment ctx
  fn construct_ctx(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> ENode<ShaderLightingGeometricCtx>;
}
pub trait LightableSurfaceProvider: ShaderPassBuilder + ShaderHashProvider {
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightableSurfaceShading>;
}
pub struct LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider<T>(pub T);
impl<T> ShaderPassBuilder for LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider<T> {}
impl<T: 'static> ShaderHashProvider
  for LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider<T>
{
  shader_hash_type_id! {}
}

impl<T: LightableSurfaceShadingLogicProvider + 'static> LightableSurfaceProvider
  for LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider<T>
{
  fn construct_shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    _: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn LightableSurfaceShading> {
    self.0.construct_shading(builder)
  }
}

pub struct DirectGeometryProvider;
impl ShaderPassBuilder for DirectGeometryProvider {}
impl ShaderHashProvider for DirectGeometryProvider {
  shader_hash_type_id! {}
}
impl GeometryCtxProvider for DirectGeometryProvider {
  fn construct_ctx(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> ENode<ShaderLightingGeometricCtx> {
    builder.fragment(|builder, _| {
      let fragment_render =
        builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();
      let fragment_normal = builder
        .query_or_interpolate_by::<FragmentRenderNormal, VertexRenderNormal>()
        .normalize();
      ENode::<ShaderLightingGeometricCtx> {
        position: fragment_render,
        normal: fragment_normal,
        view_dir: -fragment_render.normalize(),
        camera_world_position: builder.query::<CameraWorldPositionHP>(),
      }
    })
  }
}

pub struct LightingComputeComponentAsRenderComponent<'a> {
  pub scene_id: UniformBufferDataView<Vec4<u32>>,
  pub geometry_constructor: Box<dyn GeometryCtxProvider + 'a>,
  pub lighting: Box<dyn LightingComputeComponent + 'a>,
  pub surface_constructor: Box<dyn LightableSurfaceProvider + 'a>,
}

impl ShaderHashProvider for LightingComputeComponentAsRenderComponent<'_> {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.geometry_constructor.hash_type_info(hasher);
    self.lighting.hash_type_info(hasher);
    self.surface_constructor.hash_type_info(hasher);
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.geometry_constructor.hash_pipeline(hasher);
    self.lighting.hash_pipeline(hasher);
    self.surface_constructor.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for LightingComputeComponentAsRenderComponent<'_> {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.geometry_constructor.setup_pass(ctx);
    ctx.binding.bind(&self.scene_id);
    self.lighting.setup_pass(ctx);
    self.surface_constructor.setup_pass(ctx);
  }
}

impl GraphicsShaderProvider for LightingComputeComponentAsRenderComponent<'_> {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let geom_ctx = self.geometry_constructor.construct_ctx(builder);
    builder.fragment(|builder, binder| {
      let scene_id = binder.bind_by(&self.scene_id).load().x();
      let invocation = self
        .lighting
        .build_light_compute_invocation(binder, scene_id);
      let shading = self.surface_constructor.construct_shading(builder, binder);

      let hdr = invocation.compute_lights(shading.as_ref(), &geom_ctx);
      builder.register::<HDRLightResult>(hdr.diffuse + hdr.specular_and_emissive);
    })
  }
}

pub trait LightingComputeInvocation {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult>;
}

impl LightingComputeInvocation for Box<dyn LightingComputeInvocation> {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    self.as_ref().compute_lights(shading, geom_ctx)
  }
}

impl<T> LightingComputeInvocation for Node<T>
where
  Node<T>: PunctualShaderLight,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let incident = self.compute_incident_light(geom_ctx);
    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}

pub struct ShadowedPunctualLighting<L, S> {
  pub light: L,
  pub shadow: S,
}

impl<L, S> LightingComputeInvocation for ShadowedPunctualLighting<L, S>
where
  L: PunctualShaderLight,
  S: ShadowOcclusionQuery,
{
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let ShadowedPunctualLighting { light, shadow } = &self;
    let mut incident = light.compute_incident_light(geom_ctx);

    let occlusion = val(1.).make_local_var();
    if_by(incident.color.greater_than(Vec3::splat(0.)).all(), || {
      occlusion.store(shadow.query_shadow_occlusion(
        geom_ctx.position,
        geom_ctx.normal,
        geom_ctx.camera_world_position,
      ));
    });
    incident.color = incident.color * occlusion.load();

    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}
