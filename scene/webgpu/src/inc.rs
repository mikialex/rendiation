#![allow(unused_must_use)]
use std::sync::{Arc, RwLock};

use crate::*;

struct SceneGPUSystem {
  contents: Arc<GlobalGPUSystem>,
  // nodes: Vec<>
  //
}

/// The actual gpu data
struct GlobalGPUSystem {
  gpu: GPU,
  texture_2d: HashMap<usize, GPU2DTexture>,
  uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: HashMap<usize, GPUBindingSequence>,
  meshes: HashMap<usize, GPUBindingSequence>,
  models: HashMap<usize, (usize, usize)>,
}

pub enum Binding {
  Texture2D(usize),
  Uniform(TypeId, usize),
  VertexBuffer(usize),
  // draw command
}

struct GPUBindingSequence {
  bindings: Vec<Binding>,
  shader_hash: u64,
}

enum GPUBindingSequenceDelta {
  Binding,
  BindingContent,
  ShaderHash(u64),
}

struct ModelGPUBinding {
  material: usize,
  mesh: usize,
  shader_hash: u64,
}

struct StreamMap<T> {
  contents: HashMap<usize, T>,
  // waked: Arc<RwLock<Vec<usize>>>,
  // waker: Arc<RwLock<Option<Waker>>>,
}

impl<T: Stream + Unpin> Stream for StreamMap<T> {
  type Item = T::Item;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    todo!()
  }
  //
}

impl GlobalGPUSystem {
  pub fn new() -> Self {
    todo!()
  }
}
impl GlobalGPUSystem {
  fn texture2d_gpu(&self, texture2d: &SceneTexture2D) -> usize {
    //
  }

  fn material_gpu(&self, material: &SceneMaterialType) -> usize {
    //
  }

  fn mesh_gpu(&self, material: &SceneMeshType) -> usize {
    //
  }

  fn model_gpu(&self, model: &SceneModelType) -> usize {
    match model {
      SceneModelType::Standard(model) => {
        let idx = model.id();
        let model = ModelGPUBinding {
          material: todo!(),
          mesh: todo!(),
          shader_hash: todo!(),
        };
        model.listen_by(all_delta).map(|delta| match delta {
          StandardModelDelta::material(material) => {
            let idx = self.material_gpu(&material);
          }
          StandardModelDelta::mesh(mesh) => self.material_gpu(&material),
          StandardModelDelta::group(_) => todo!(),
          StandardModelDelta::skeleton(_) => todo!(),
        });
        todo!()
        //
      }
      SceneModelType::Foreign(_) => todo!(),
      _ => todo!(),
    }
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
