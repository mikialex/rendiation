use crate::*;
use std::sync::{Arc, RwLock};

// struct SceneNodeGPUSystem;
// struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

pub struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
  // nodes: SceneNodeGPUSystem,
  // // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  // cameras: SceneCameraGPUSystem,
  // bundle: SceneBundleGPUSystem,
  models: StreamMap<SceneModelGPUReactive>,
  source: SceneGPUUpdateSource,
}

impl SceneGPUSystem {
  pub fn render(&self, encoder: &mut GPUCommandEncoder, pass_dispatcher: &dyn RenderComponent) {
    // do encoding
  }
}

impl Stream for SceneGPUSystem {
  type Item = ();

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    todo!()
  }
}
type SceneGPUUpdateSource = impl Stream;

impl SceneGPUSystem {
  pub fn new(scene: &Scene, contents: Arc<RwLock<GlobalGPUSystem>>) -> Self {
    let models = Default::default();
    let contents_c = contents.clone();

    let source = scene.listen_by(all_delta).map(move |delta| {
      let contents = contents_c.write().unwrap();
      let mut models = contents.models.write().unwrap();
      match delta {
        SceneInnerDelta::models(delta) => match delta {
          arena::ArenaDelta::Mutate((model, _)) => {
            models.remove(model.id());
            models.get_or_insert_with(model.id(), || {
              //
              todo!()
            });
          }
          arena::ArenaDelta::Insert((model, _)) => {
            models.get_or_insert_with(model.id(), || {
              //
              todo!()
            });
          }
          arena::ArenaDelta::Remove(index) => {
            // models.remove(model.id());
          }
        },
        _ => {}
      }
    });

    Self {
      contents,
      models,
      // nodes: (),
      // cameras: (),
      // bundle: (),
      source,
    }
  }

  pub fn maintain(&mut self) {
    //
  }

  pub fn render_with_dispatcher(&self, dispatcher: &dyn RenderComponent) -> webgpu::CommandBuffer {
    todo!()
  }
}
