use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
}

impl ShaderLight for DirectionalLightShaderInfo {
  fn name() -> &'static str {
    "directional_light"
  }
  type Dependency = ();
  fn create_dep(_: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency {}
  fn compute_direct_light(
    node: &ExpandedNode<Self>,
    _dep: &Self::Dependency,
    _ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight> {
    //
    ExpandedNode::<ShaderIncidentLight> {
      color: node.intensity,
      direction: node.direction,
    }
  }
}

impl WebGPUSceneLight for SceneItemRef<DirectionalLight> {
  fn check_update_gpu<'a>(&self, res: &'a mut ForwardLightingSystem, gpu: &GPU) {
    let lights = res
      .lights_collections
      .entry(self.type_id())
      .or_insert_with(|| todo!());
    // let lights = lights
    //   .downcast_mut::<LightList<DirectionalLightShaderInfo>>()
    //   .unwrap();
  }
}
