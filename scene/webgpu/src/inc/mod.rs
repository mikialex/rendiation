#![allow(unused_must_use)]
use std::sync::{Arc, RwLock};

use crate::*;

mod deltas;
pub use deltas::*;

struct SceneNodeGPUSystem;
struct SceneCameraGPUSystem;
struct SceneBundleGPUSystem;

struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
  nodes: SceneNodeGPUSystem,
  // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  cameras: SceneCameraGPUSystem,
  bundle: SceneBundleGPUSystem,
}

impl SceneGPUSystem {
  pub fn render(pass_dispatcher: &dyn RenderComponent) {
    // do submit
  }
}

#[derive(Clone)]
pub struct ResourceGPUCtx {
  pub device: GPUDevice,
  pub queue: GPUQueue,
  pub mipmap_gen: Rc<RefCell<MipMapTaskManager>>,
}

/// The actual gpu data
struct GlobalGPUSystem {
  gpu: ResourceGPUCtx,
  shared: ShareBindableResource,
  // uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: StreamMap<ReactiveRenderComponent>,
  meshes: StreamMap<ReactiveRenderComponent>,
  models: StreamMap<ReactiveRenderComponent>,
}

impl GlobalGPUSystem {
  pub fn new() -> Self {
    todo!()
  }
}

pub struct ShareBindableResource {
  pub gpu: ResourceGPUCtx,
  pub texture_2d: StreamMap<ReactiveGPU2DTextureViewBuilder>,
  // texture_cube
  // any shared uniforms
}

pub struct WhichModelRenderContentChange;

impl Stream for GlobalGPUSystem {
  type Item = WhichModelRenderContentChange;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    // models are root, only poll model
    todo!()
  }
}

pub type ReactiveRenderComponent =
  impl Stream<Item = RenderComponentDelta> + AsRef<dyn RenderComponent>;

fn standard_model(model: &SceneItemRef<StandardModel>) -> StandardModelGPUReactive {
  let m = todo!();
  model
    .listen_by(all_delta)
    .fold_signal(m, |delta, m: &mut ModelGPUBinding| {
      //
      ()
    })
}

pub enum ModelGPUReactive {
  Standard(),
  Foreign,
}

impl GlobalGPUSystem {
  fn texture2d_gpu(&self, texture2d: &SceneTexture2D) -> usize {
    todo!()
  }

  fn material_gpu(&self, material: &SceneMaterialType) -> usize {
    match material {
      SceneMaterialType::PhysicalSpecularGlossiness(material) => {
        let binding = GPUBindingSequence {
          bindings: todo!(),
          shader_hash: todo!(),
        };
      }
      SceneMaterialType::PhysicalMetallicRoughness(_) => todo!(),
      SceneMaterialType::Flat(_) => todo!(),
      SceneMaterialType::Foreign(_) => todo!(),
      _ => todo!(),
    }
  }

  fn mesh_gpu(&self, material: &SceneMeshType) -> usize {
    todo!()
  }

  fn model_gpu(&self, model: &SceneModelType) -> ModelGPUReactive {
    todo!()

    // match model {
    //   SceneModelType::Standard(model) => {
    //     let idx = model.id();
    //     if todo!() {
    //       return idx;
    //     }
    //     let m = model.read();
    //     let gpu_model = ModelGPUBinding {
    //       material: self.material_gpu(&m.material),
    //       mesh: self.mesh_gpu(&m.mesh),
    //       shader_hash: todo!(),
    //     };
    //     let stream = model.listen_by(all_delta).map(|delta| match delta {
    //       StandardModelDelta::material(material) => self
    //         .material_gpu(&material)
    //         .wrap(ModelGPUBindingDelta::material),
    //       StandardModelDelta::mesh(mesh) => self.mesh_gpu(&mesh).wrap(ModelGPUBindingDelta::mesh),
    //       StandardModelDelta::group(group) => todo!(),
    //       StandardModelDelta::skeleton(_) => todo!(),
    //     });

    //     self.models.insert(idx, (gpu_model, stream));
    //     idx
    //   }
    //   SceneModelType::Foreign(_) => todo!(),
    //   _ => todo!(),
    // }
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

fn create_material_gpu(
  sys: &GlobalGPUSystem,
  material: &SceneMaterialType,
  res: &mut ShareBindableResource,
) -> MaterialGPUReactive {
  match material {
    SceneMaterialType::PhysicalSpecularGlossiness(_) => todo!(),
    SceneMaterialType::PhysicalMetallicRoughness(m) => {
      MaterialGPUReactive::PhysicalMetallicRoughnessMaterialGPU(
        create_physical_metallic_material_gpu(m, res, gpu),
      )
    }
    SceneMaterialType::Flat(_) => todo!(),
    SceneMaterialType::Foreign(_) => todo!(),
    _ => todo!(),
  }
}

pub enum MaterialGPUReactive {
  PhysicalMetallicRoughnessMaterialGPU(ReactivePhysicalMetallicRoughnessMaterialGPU),
  Foreign,
}

impl MaterialGPUReactive {
  pub fn as_render_component(&self) -> &dyn RenderComponent {
    match self {
      MaterialGPUReactive::PhysicalMetallicRoughnessMaterialGPU(gpu) => {
        gpu.as_ref() as &dyn RenderComponent
      }
      MaterialGPUReactive::Foreign => &(),
    }
  }
}
