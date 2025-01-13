use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightUniform {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

pub fn point_uniform_array(gpu: &GPU) -> UniformArrayUpdateContainer<PointLightUniform> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch::<PointLightIntensity>()
    .into_query_update_uniform_array(offset_of!(PointLightUniform, luminance_intensity), gpu);

  let cutoff_distance = global_watch()
    .watch::<PointLightCutOffDistance>()
    .into_query_update_uniform_array(offset_of!(PointLightUniform, cutoff_distance), gpu);

  let position = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<PointLightRefNode>())
    .collective_map(|mat| mat.position())
    .into_query_update_uniform_array(offset_of!(PointLightUniform, position), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(position)
}

#[derive(Default)]
pub struct PointLightUniformLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for PointLightUniformLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniform = point_uniform_array(cx);
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
      .take_multi_updater_updated::<UniformArray<PointLightUniform, 8>>(self.token)
      .unwrap()
      .target
      .clone();

    Box::new(ScenePointLightingProvider { uniform })
  }
}

struct ScenePointLightingProvider {
  uniform: UniformBufferDataView<Shader140Array<PointLightUniform, 8>>,
}

impl LightSystemSceneProvider for ScenePointLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.uniform.clone(),
      |(_, light_uniform): (Node<u32>, UniformNode<PointLightUniform>)| {
        let light_uniform = light_uniform.load().expand();
        ENode::<PointLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          cutoff_distance: light_uniform.cutoff_distance,
        }
        .construct()
      },
    );
    Some(Box::new(com))
  }
}
