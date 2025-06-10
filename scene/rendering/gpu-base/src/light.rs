use rendiation_lighting_gpu_system::*;

use crate::*;

pub trait LightSystemSceneProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>>;
}

// #[derive(Default)]
// pub struct DifferentLightRenderImplProvider {
//   lights: Vec<BoxedQueryBasedGPUFeature<Box<dyn LightSystemSceneProvider>>>,
// }

// impl DifferentLightRenderImplProvider {
//   pub fn with_light(
//     mut self,
//     impls: impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>, Context = GPU> + 'static,
//   ) -> Self {
//     self.lights.push(Box::new(impls));
//     self
//   }
// }

// impl QueryBasedFeature<Box<dyn LightSystemSceneProvider>> for DifferentLightRenderImplProvider {
//   type Context = GPU;
//   fn register(&mut self, qcx: &mut ReactiveQueryCtx, cx: &GPU) {
//     self.lights.iter_mut().for_each(|l| l.register(qcx, cx));
//   }
//   fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
//     self.lights.iter_mut().for_each(|l| l.deregister(qcx));
//   }

//   fn create_impl(&self, cx: &mut QueryResultCtx) -> Box<dyn LightSystemSceneProvider> {
//     Box::new(LightingComputeComponentGroupProvider {
//       lights: self.lights.iter().map(|i| i.create_impl(cx)).collect(),
//     })
//   }
// }

struct LightingComputeComponentGroupProvider {
  lights: Vec<Box<dyn LightSystemSceneProvider>>,
}

impl LightSystemSceneProvider for LightingComputeComponentGroupProvider {
  fn get_scene_lighting(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> Option<Box<dyn LightingComputeComponent>> {
    Some(Box::new(LightingComputeComponentGroup {
      comps: self
        .lights
        .iter()
        .filter_map(|i| i.get_scene_lighting(scene))
        .collect(),
    }))
  }
}
