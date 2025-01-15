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

pub struct LightingComputeComponentAsRenderComponent(pub Box<dyn LightingComputeComponent>);

impl ShaderHashProvider for LightingComputeComponentAsRenderComponent {
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_type_info(hasher)
  }
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for LightingComputeComponentAsRenderComponent {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.setup_pass(ctx);
  }
}

impl GraphicsShaderProvider for LightingComputeComponentAsRenderComponent {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binder| {
      let invocation = self.0.build_light_compute_invocation(binder);
      let shading_provider = PhysicalShading; // todo, make it configurable by user
      let shading = shading_provider.construct_shading(builder); // todo, make it return optional to avoid lighting cost for none lightable material

      let fragment_world =
        builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();
      let camera_position = builder.query::<CameraWorldMatrix>().position();
      let geom_ctx = ENode::<ShaderLightingGeometricCtx> {
        position: fragment_world,
        normal: builder.query_or_interpolate_by::<FragmentWorldNormal, WorldVertexNormal>(),
        view_dir: (camera_position - fragment_world).normalize(),
      };

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
