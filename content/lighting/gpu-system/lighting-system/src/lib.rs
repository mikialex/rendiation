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
  ) -> Box<dyn LightingComputeInvocation>;
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx);
}

pub trait GeometryCtxProvider {
  fn construct_ctx(
    &self,
    builder: &mut ShaderFragmentBuilderView,
  ) -> ENode<ShaderLightingGeometricCtx>;
}

pub struct DirectGeometryProvider;

impl GeometryCtxProvider for DirectGeometryProvider {
  fn construct_ctx(
    &self,
    builder: &mut ShaderFragmentBuilderView,
  ) -> ENode<ShaderLightingGeometricCtx> {
    let fragment_world =
      builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();
    let fragment_normal =
      builder.query_or_interpolate_by::<FragmentWorldNormal, WorldVertexNormal>();
    let camera_position = builder.query::<CameraWorldMatrix>().position();
    ENode::<ShaderLightingGeometricCtx> {
      position: fragment_world,
      normal: fragment_normal,
      view_dir: (camera_position - fragment_world).normalize(),
    }
  }
}

// todo, merge this with the forward one
pub struct LightingComputeComponentAsRenderComponent {
  pub geometry_constructor: Box<dyn GeometryCtxProvider>, // todo, this should be hashed
  pub surface_constructor: Box<dyn LightableSurfaceShadingProvider>, // todo, this should be hashed
  pub lighting: Box<dyn LightingComputeComponent>,
}

impl ShaderHashProvider for LightingComputeComponentAsRenderComponent {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.lighting.hash_type_info(hasher)
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.lighting.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for LightingComputeComponentAsRenderComponent {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.lighting.setup_pass(ctx);
  }
}

impl GraphicsShaderProvider for LightingComputeComponentAsRenderComponent {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binder| {
      let invocation = self.lighting.build_light_compute_invocation(binder);
      let shading = self.surface_constructor.construct_shading(builder);
      let geom_ctx = self.geometry_constructor.construct_ctx(builder);

      let hdr = invocation.compute_lights(shading.as_ref(), &geom_ctx);
      builder.register::<HDRLightResult>(hdr.diffuse + hdr.specular);
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
      occlusion.store(shadow.query_shadow_occlusion(geom_ctx.position, geom_ctx.normal));
    });
    incident.color = incident.color * occlusion.load();

    shading.compute_lighting_by_incident(&incident, geom_ctx)
  }
}
