use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub node_info: TransformGPUData, // maybe used later in shadowmap
}

impl ShaderLight for DirectionalLightShaderInfo {
  type Dependency = ();
  fn create_dep(_: &mut ShaderGraphFragmentBuilderView) -> Self::Dependency {}
  fn compute_direct_light(
    light: &ExpandedNode<Self>,
    _dep: &Self::Dependency,
    _ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderIncidentLight> {
    //
    ExpandedNode::<ShaderIncidentLight> {
      color: light.intensity,
      direction: light.direction,
    }
  }
}

impl WebGPUSceneLight for DirectionalLight {
  fn collect(&self, sys: &mut ForwardLightingSystem, node: &SceneNode) {
    let lights = sys.get_or_create_list();

    let gpu = DirectionalLightShaderInfo {
      intensity: self.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      node_info: TransformGPUData::from_node(node, None),
      ..Zeroable::zeroed()
    };

    lights.lights.push(gpu)
  }
}
