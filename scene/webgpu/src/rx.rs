use crate::*;

pub struct WebGPUSceneSystem {
  // nodes: IdentityMapper<>
  // camera: IdentityMapper<>
}

impl WebGPUSceneSystem {
  pub fn build(scene: &Scene) -> Self {
    scene.read().delta_stream().on(|delta| {
      match delta {
        SceneInnerDelta::background(_) => todo!(),
        SceneInnerDelta::default_camera(_) => todo!(),
        SceneInnerDelta::active_camera(_) => todo!(),
        SceneInnerDelta::cameras(camera_delta) => todo!(),
        SceneInnerDelta::lights(_) => todo!(),
        SceneInnerDelta::models(model_delta) => match model_delta {
          arena::ArenaDelta::Mutate(_) => todo!(),
          arena::ArenaDelta::Insert(new_model) => {
            //
          }
          arena::ArenaDelta::Remove(_) => todo!(),
        },
        SceneInnerDelta::ext(_) => todo!(),
      }
    })
  }

  pub fn render() {
    //
  }
}
