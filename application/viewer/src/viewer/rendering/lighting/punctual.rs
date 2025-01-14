use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::BasicShadowMapInfo;
use rendiation_webgpu_reactive_utils::{UniformArray, UniformArrayUpdateContainer};

use crate::*;

pub struct DirectionalShaderAtlas(pub GPUTexture);
pub struct SpotShaderAtlas(pub GPUTexture);

pub struct DirectionalUniformLightList {
  light: UpdateResultToken,
  shadow: UpdateResultToken,
}

impl DirectionalUniformLightList {
  pub fn new(
    source: &mut ReactiveQueryJoinUpdater,
    light: UniformArrayUpdateContainer<DirectionalLightUniform>,
    shadow: UniformArrayUpdateContainer<BasicShadowMapInfo>,
  ) -> Self {
    Self {
      light: source.register_multi_updater(light),
      shadow: source.register_multi_updater(shadow),
    }
  }
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for DirectionalUniformLightList {
  // registered in constructor
  fn register_resource(&mut self, _: &mut ReactiveQueryJoinUpdater, _: &GPU) {}

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.light);
    source.deregister(&mut self.shadow);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let light = res
      .take_multi_updater_updated::<UniformArray<DirectionalLightUniform, 8>>(self.light)
      .unwrap()
      .target
      .clone();

    let shadow = res
      .take_multi_updater_updated::<UniformArray<BasicShadowMapInfo, 8>>(self.shadow)
      .unwrap()
      .target
      .clone();
    Box::new(SceneDirectionalLightingProvider {
      light,
      shadow,
      map: res
        .type_based_result
        .take::<DirectionalShaderAtlas>()
        .unwrap()
        .0,
    })
  }
}

struct SceneDirectionalLightingProvider {
  light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
  shadow: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
  map: GPUTexture,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.light.clone(),
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
  light: UpdateResultToken,
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for PointLightUniformLightList {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let uniform = point_uniform_array(cx);
    self.light = source.register_multi_updater(uniform);
  }
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.light);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let uniform = res
      .take_multi_updater_updated::<UniformArray<PointLightUniform, 8>>(self.light)
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

pub struct SpotLightUniformLightList {
  light: UpdateResultToken,
  shadow: UpdateResultToken,
}

impl SpotLightUniformLightList {
  pub fn new(
    source: &mut ReactiveQueryJoinUpdater,
    light: UniformArrayUpdateContainer<SpotLightUniform>,
    shadow: UniformArrayUpdateContainer<BasicShadowMapInfo>,
  ) -> Self {
    Self {
      light: source.register_multi_updater(light),
      shadow: source.register_multi_updater(shadow),
    }
  }
}

impl RenderImplProvider<Box<dyn LightSystemSceneProvider>> for SpotLightUniformLightList {
  // registered in constructor
  fn register_resource(&mut self, _source: &mut ReactiveQueryJoinUpdater, _cx: &GPU) {}
  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.light);
    source.deregister(&mut self.shadow);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let light = res
      .take_multi_updater_updated::<UniformArray<SpotLightUniform, 8>>(self.light)
      .unwrap()
      .target
      .clone();

    let shadow = res
      .take_multi_updater_updated::<UniformArray<BasicShadowMapInfo, 8>>(self.shadow)
      .unwrap()
      .target
      .clone();
    Box::new(SceneSpotLightingProvider {
      light,
      shadow,
      map: res.type_based_result.take::<SpotShaderAtlas>().unwrap().0,
    })
  }
}

struct SceneSpotLightingProvider {
  light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
  shadow: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
  map: GPUTexture,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      self.light.clone(),
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
