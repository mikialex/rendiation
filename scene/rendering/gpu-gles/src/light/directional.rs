use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightUniform {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn directional_uniform_array(
  gpu: &GPU,
) -> UniformArrayUpdateContainer<DirectionalLightUniform> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let illuminance = global_watch()
    .watch::<DirectionalLightIlluminance>()
    .into_query_update_uniform_array(offset_of!(DirectionalLightUniform, illuminance), gpu);

  let direction = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_query_update_uniform_array(offset_of!(DirectionalLightUniform, direction), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(illuminance)
    .with_source(direction)
}

#[derive(Default)]
pub struct DirectionalUniformLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for DirectionalUniformLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniform = directional_uniform_array(cx);
    self.token = source.register_multi_updater(uniform);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  fn create_impl(
    &self,
    res: &mut ConcurrentStreamUpdateResult,
  ) -> Box<dyn LightSystemSceneProvider> {
    let uniform = res
      .take_multi_updater_updated::<UniformArray<DirectionalLightUniform, 8>>(self.token)
      .unwrap()
      .target
      .clone();
    Box::new(SceneDirectionalLightingProvider { uniform })
  }
}

struct SceneDirectionalLightingProvider {
  uniform: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.uniform.clone(),
      |(_, light_uniform): (Node<u32>, UniformNode<DirectionalLightUniform>)| {
        let light_uniform = light_uniform.load().expand();
        ENode::<DirectionalShaderInfo> {
          illuminance: light_uniform.illuminance,
          direction: light_uniform.direction,
        }
        .construct()
      },
    );
    Some(Box::new(com))
  }
}
