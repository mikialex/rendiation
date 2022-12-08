use crate::*;

pub struct WebGPUSceneSystem {
  // nodes: IdentityMapper<>
  camera: IdentityMapper<CameraGPU, SceneCameraInner>,
  foreign_model: IdentityMapper<Box<dyn RenderComponentAny>, Box<dyn SceneRenderable>>,
  materials: IdentityMapper<Box<dyn RenderComponentAny>, SceneMaterialType>,
  meshes: IdentityMapper<Box<dyn RenderComponentAny>, SceneMeshType>,
}

impl WebGPUSceneSystem {
  pub fn build(scene: &Scene) -> Self {
    let scene = scene.read();

    let deltas = scene.read().delta_stream();

    let system = deltas.fold(todo!(), |delta, system: &mut WebGPUSceneSystem| {
      match delta {
        SceneInnerDelta::background(background) => match background {
          Some(_) => todo!(),
          None => todo!(),
        },
        SceneInnerDelta::default_camera(_) => todo!(),
        SceneInnerDelta::active_camera(_) => todo!(),
        SceneInnerDelta::cameras(camera_delta) => match camera_delta {
          arena::ArenaDelta::Mutate((camera, _)) => {
            // todo handle switch
          }
          arena::ArenaDelta::Insert((camera, _)) => {
            system.camera.insert(camera);
          }
          arena::ArenaDelta::Remove(camera) => {
            system.camera.remove(camera);
          }
        },
        SceneInnerDelta::lights(light_delta) => match light_delta {
          arena::ArenaDelta::Mutate(_) => todo!(),
          arena::ArenaDelta::Insert(_) => todo!(),
          arena::ArenaDelta::Remove(_) => todo!(),
        },
        SceneInnerDelta::models(model_delta) => match model_delta {
          arena::ArenaDelta::Mutate(_) => todo!(),
          arena::ArenaDelta::Insert((new_model, _)) => {
            let model = new_model.read();
            match model.model {
              SceneModelType::Standard(model) => {
                let model = model.read();

                let material = model.material;
                system.materials.insert(material);

                let mesh = model.mesh;
                system.meshes.insert(mesh);
              }
              SceneModelType::Foreign(foreign) => {
                if let Some(model) = model.downcast_ref::<Box<dyn SceneRenderable>>() {
                  // model.render(pass, dispatcher, camera)
                }
              }
              _ => todo!(),
            }
          }
          arena::ArenaDelta::Remove(_) => todo!(),
        },
        SceneInnerDelta::ext(_) => todo!(),
      }
    });

    // trigger the first synchronization
    scene.expand(|delta| deltas.emit(delta));

    system
  }

  pub fn render() {
    //
  }
}
