use crate::*;
use std::sync::{Arc, RwLock};

// struct SceneNodeGPUSystem;
// struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
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
    scene.listen_by(all_delta).map(|delta| match delta {
      SceneInnerDelta::models(delta) => match delta {
        arena::ArenaDelta::Mutate(_) => todo!(),
        arena::ArenaDelta::Insert((model, _)) => {
          model.listen_by(all_delta).map(|delta| match delta {
            SceneModelImplDelta::model(model) => match model {
              SceneModelType::Standard(model) => {
                model.listen_by(all_delta).map(|delta| match delta {
                  StandardModelDelta::material(material) => match material {
                    SceneMaterialType::PhysicalSpecularGlossiness(_) => todo!(),
                    SceneMaterialType::PhysicalMetallicRoughness(_) => todo!(),
                    SceneMaterialType::Flat(_) => todo!(),
                    SceneMaterialType::Foreign(_) => todo!(),
                    _ => todo!(),
                  },
                  StandardModelDelta::mesh(_) => todo!(),
                  StandardModelDelta::group(_) => todo!(),
                  StandardModelDelta::skeleton(_) => todo!(),
                });
              }
              SceneModelType::Foreign(_) => todo!(),
              _ => todo!(),
            },
            SceneModelImplDelta::node(_) => todo!(),
          });
        }
        arena::ArenaDelta::Remove(_) => todo!(),
      },
      _ => {}
    });
    Self {
      contents,
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
