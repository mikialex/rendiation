use rendiation_lighting_punctual::*;
use rendiation_webgpu_reactive_utils::UniformArray;

use crate::*;

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

#[derive(Default)]
pub struct SpotLightUniformLightList {
  token: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for SpotLightUniformLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniform = spot_uniform_array(cx);
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
      .take_multi_updater_updated::<UniformArray<SpotLightUniform, 8>>(self.token)
      .unwrap()
      .target
      .clone();
    Box::new(SceneSpotLightingProvider { uniform })
  }
}

struct SceneSpotLightingProvider {
  uniform: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.uniform.clone(),
      |(_, light_uniform): (Node<u32>, UniformNode<SpotLightUniform>)| {
        let light_uniform = light_uniform.load().expand();
        ENode::<SpotLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          direction: light_uniform.direction,
          cutoff_distance: light_uniform.cutoff_distance,
          half_cone_cos: light_uniform.half_cone_cos,
          half_penumbra_cos: light_uniform.half_penumbra_cos,
        }
        .construct()
      },
    );
    Some(Box::new(com))
  }
}
