use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct PhysicalShading {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub roughness: f32,
}

both!(ColorChannel, Vec3<f32>);
both!(SpecularChannel, Vec3<f32>);
both!(RoughnessChannel, f32);

impl LightableSurfaceShading for PhysicalShading {
  fn construct(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<Self> {
    ExpandedNode::<Self> {
      diffuse: builder.query_or_insert_default::<ColorChannel>().get(),
      specular: builder.query_or_insert_default::<SpecularChannel>().get(),
      roughness: builder.query_or_insert_default::<RoughnessChannel>().get(),
    }
  }

  fn compute_lighting(
    self_node: &ExpandedNode<Self>,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult> {
    todo!()
  }
}
