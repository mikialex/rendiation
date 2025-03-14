use rendiation_lighting_punctual::*;
use rendiation_lighting_shadow_map::*;
use rendiation_webgpu_reactive_utils::{UniformArray, UniformArrayUpdateContainer};

use crate::*;

pub struct DirectionalShaderAtlas(pub GPU2DArrayDepthTextureView);
pub struct SpotShaderAtlas(pub GPU2DArrayDepthTextureView);

pub struct DirectionalUniformLightList {
  light: QueryToken,
  shadow: QueryToken,
  reversed_depth: bool,
}

impl DirectionalUniformLightList {
  pub fn new(
    qcx: &mut ReactiveQueryCtx,
    light: UniformArrayUpdateContainer<DirectionalLightUniform, 8>,
    shadow: UniformArrayUpdateContainer<BasicShadowMapInfo, 8>,
    reversed_depth: bool,
  ) -> Self {
    Self {
      light: qcx.register_multi_updater(light),
      shadow: qcx.register_multi_updater(shadow),
      reversed_depth,
    }
  }
}

impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>> for DirectionalUniformLightList {
  type Context = GPU;
  // already registered in constructor
  fn register(&mut self, _: &mut ReactiveQueryCtx, _: &GPU) {}

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.light);
    qcx.deregister(&mut self.shadow);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let light = cx
      .take_multi_updater_updated::<UniformArray<DirectionalLightUniform, 8>>(self.light)
      .unwrap()
      .target
      .clone();

    let info = cx
      .take_multi_updater_updated::<UniformArray<BasicShadowMapInfo, 8>>(self.shadow)
      .unwrap()
      .target
      .clone();
    let shadow_map_atlas = cx
      .type_based_result
      .take::<DirectionalShaderAtlas>()
      .unwrap()
      .0;
    Box::new(SceneDirectionalLightingProvider {
      light,
      shadow: BasicShadowMapComponent {
        shadow_map_atlas,
        info,
        reversed_depth: self.reversed_depth,
      },
    })
  }
}

struct SceneDirectionalLightingProvider {
  light: UniformBufferDataView<Shader140Array<DirectionalLightUniform, 8>>,
  shadow: BasicShadowMapComponent,
}

impl LightSystemSceneProvider for SceneDirectionalLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      LightAndShadowCombinedSource(self.light.clone(), self.shadow.clone()),
      |((_, light), shadow): (
        (Node<u32>, ShaderReadonlyPtrOf<DirectionalLightUniform>),
        BasicShadowMapSingleInvocation,
      )| {
        let light_uniform = light.load().expand();
        let light = ENode::<DirectionalShaderInfo> {
          illuminance: light_uniform.illuminance,
          direction: light_uniform.direction,
        }
        .construct();
        ShadowedPunctualLighting { light, shadow }
      },
    );
    Some(Box::new(com))
  }
}

#[derive(Default)]
pub struct PointLightUniformLightList {
  light: QueryToken,
}

impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>> for PointLightUniformLightList {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
    let uniform = point_uniform_array(cx);
    self.light = qcx.register_multi_updater(uniform);
  }
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.light);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let uniform = cx
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
      |(_, light_uniform): (Node<u32>, ShaderReadonlyPtrOf<PointLightUniform>)| {
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
  light: QueryToken,
  shadow: QueryToken,
  reversed_depth: bool,
}

impl SpotLightUniformLightList {
  pub fn new(
    qcx: &mut ReactiveQueryCtx,
    light: UniformArrayUpdateContainer<SpotLightUniform, 8>,
    shadow: UniformArrayUpdateContainer<BasicShadowMapInfo, 8>,
    reversed_depth: bool,
  ) -> Self {
    Self {
      light: qcx.register_multi_updater(light),
      shadow: qcx.register_multi_updater(shadow),
      reversed_depth,
    }
  }
}

impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>> for SpotLightUniformLightList {
  type Context = GPU;
  // registered in constructor
  fn register(&mut self, _qcx: &mut ReactiveQueryCtx, _cx: &GPU) {}
  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.light);
    qcx.deregister(&mut self.shadow);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
    let light = cx
      .take_multi_updater_updated::<UniformArray<SpotLightUniform, 8>>(self.light)
      .unwrap()
      .target
      .clone();
    let info = cx
      .take_multi_updater_updated::<UniformArray<BasicShadowMapInfo, 8>>(self.shadow)
      .unwrap()
      .target
      .clone();
    let shadow_map_atlas = cx.type_based_result.take::<SpotShaderAtlas>().unwrap().0;
    Box::new(SceneSpotLightingProvider {
      light,
      shadow: BasicShadowMapComponent {
        shadow_map_atlas,
        info,
        reversed_depth: self.reversed_depth,
      },
    })
  }
}

struct SceneSpotLightingProvider {
  light: UniformBufferDataView<Shader140Array<SpotLightUniform, 8>>,
  shadow: BasicShadowMapComponent,
}

impl LightSystemSceneProvider for SceneSpotLightingProvider {
  fn get_scene_lighting(
    &self,
    _scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    let com = ArrayLights(
      LightAndShadowCombinedSource(self.light.clone(), self.shadow.clone()),
      |((_, light), shadow): (
        (Node<u32>, ShaderReadonlyPtrOf<SpotLightUniform>),
        BasicShadowMapSingleInvocation,
      )| {
        let light_uniform = light.load().expand();
        let light = ENode::<SpotLightShaderInfo> {
          luminance_intensity: light_uniform.luminance_intensity,
          position: light_uniform.position,
          direction: light_uniform.direction,
          cutoff_distance: light_uniform.cutoff_distance,
          half_cone_cos: light_uniform.half_cone_cos,
          half_penumbra_cos: light_uniform.half_penumbra_cos,
        }
        .construct();
        ShadowedPunctualLighting { light, shadow }
      },
    );

    Some(Box::new(com))
  }
}
