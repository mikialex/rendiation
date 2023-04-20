use crate::*;
use std::sync::{Arc, RwLock};

// struct SceneNodeGPUSystem;
// struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

pub struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
  models: Arc<RwLock<StreamMap<ModelGPUReactive>>>,
  // nodes: SceneNodeGPUSystem,
  // // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  // cameras: SceneCameraGPUSystem,
  // bundle: SceneBundleGPUSystem,
}

impl SceneGPUSystem {
  pub fn render(pass_dispatcher: &dyn RenderComponent) {
    // do submit
  }
}

impl Stream for SceneGPUSystem {
  type Item = ();

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    // models are root, only poll model
    todo!()
  }
}

impl SceneGPUSystem {
  pub fn new(scene: &Scene, contents: Arc<RwLock<GlobalGPUSystem>>) -> Self {
    let models: Arc<RwLock<StreamMap<ModelGPUReactive>>> = Default::default();
    let models_c = models.clone();
    scene.listen_by(all_delta).map(|delta| {
      let contents = contents.write().unwrap();
      let mut models = models_c.write().unwrap();
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
    }
  }
  pub fn maintain(&mut self) {
    //
  }

  pub fn render_with_dispatcher(&self, dispatcher: &dyn RenderComponent) -> webgpu::CommandBuffer {
    todo!()
  }
}
