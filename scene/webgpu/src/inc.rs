use std::sync::Arc;

use crate::*;

struct SceneGPUSystem {
  contents: Arc<GlobalGPUSystem>,
  // nodes: Vec<>
  //
}

// enum ResourceChange{

// }

/// The actual gpu data
struct GlobalGPUSystem {
  gpu: GPU,
  // material_uniforms:
  // textures:
  // mesh_buffers:
}

impl GlobalGPUSystem {
  pub fn new() -> Self {
    todo!()
  }
}

impl SceneGPUSystem {
  pub fn new(scene: &Scene, contents: GlobalGPUSystem) -> Self {
    scene.listen_by(all_delta).map(|delta| match delta {
      SceneInnerDelta::background(_) => todo!(),
      SceneInnerDelta::default_camera(_) => todo!(),
      SceneInnerDelta::active_camera(_) => todo!(),
      SceneInnerDelta::cameras(_) => todo!(),
      SceneInnerDelta::lights(_) => todo!(),
      SceneInnerDelta::models(delta) => {
        match delta {
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
        }
        //
        // contents.insert()
      }
      SceneInnerDelta::ext(_) => todo!(),
      SceneInnerDelta::nodes(_) => todo!(),
    });
    todo!()
  }
  pub fn maintain(&mut self) {
    //
  }

  pub fn render_with_dispatcher(&self, dispatcher: &dyn RenderComponent) -> webgpu::CommandBuffer {
    todo!()
  }
}
