#![allow(unused_must_use)]
use std::sync::{Arc, RwLock};

use crate::*;

struct SceneNodeGPUSystem;
struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
  nodes: SceneNodeGPUSystem,
  // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  cameras: SceneCameraGPUSystem,
  // bundle: SceneBundleGPUSystem,
}

impl SceneGPUSystem {
  pub fn render(pass_dispatcher: &dyn RenderComponent) {
    // do submit
  }
}

pub enum GPUResourceChange {
  Reference,
  Content,
}

#[derive(Clone)]
struct GPUCtx;

/// The actual gpu data
struct GlobalGPUSystem {
  gpu: GPUCtx,
  shared: ShareBindableResource,
  // uniforms: HashMap<TypeId, Box<dyn Any>>,
  materials: StreamMap<GPUBindingSequenceReactive>,
  meshes: StreamMap<GPUBindingSequenceReactive>,
  // models: StreamMap<ModelGPUBindingReactive>,
}

pub struct ShareBindableResource {
  texture_2d: StreamMap<ReactiveGPU2DTextureView>,
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

pub enum Binding {
  Texture2D(usize),
  Uniform(TypeId, usize),
  VertexBuffer(usize),
  // draw command
}

pub type GPUBindingSequenceReactive =
  impl Stream<Item = GPUBindingSequenceDelta> + AsRef<GPUBindingSequence>;

/// could just the product of shader hash and shader pass builder
struct GPUBindingSequence {
  bindings: Vec<Binding>, // use small vec
  shader_hash: u64,
}
// clone_self_incremental!(GPUBindingSequence);

enum GPUBindingSequenceDelta {
  Binding,
  BindingContent,
  ShaderHash(u64),
}

// pub type ModelGPUBindingReactive =
//   impl Stream<Item = ModelGPUBindingReactiveDelta> + AsRef<ModelGPUBindingReactive>;
#[derive(Incremental)]
struct ModelGPUBinding {
  pub material: usize,
  pub mesh: usize,
  pub shader_hash: u64,
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

pub type StandardModelGPUReactive = impl Stream + Unpin + AsRef<ModelGPUBinding>;

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
  material: &SceneMaterialType,
  res: &mut ShareBindableResource,
  gpu: &GPUCtx,
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

pub type ReactivePhysicalMetallicRoughnessMaterialGPU =
  impl Stream<Item = GPUResourceChange> + Unpin + AsRef<PhysicalMetallicRoughnessMaterialGPU>;

#[pin_project(project = MaterialGPUChangeProj)]
pub enum MaterialGPUChange<T> {
  Texture(T, #[pin] ReactiveGPU2DTextureView),
  // Uniform(T,),  we don't have shared uniforms now
  OwnedBindingContent,
  OwnedBindingRef(T),
  ShaderHash,
}

pub enum MaterialGPUChangeFlattened<T> {
  ContentRef(T),
  Content,
  ShaderHash,
}

impl<T: Copy> Stream for MaterialGPUChange<T> {
  type Item = MaterialGPUChangeFlattened<T>;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    Poll::Ready(Some(match self.project() {
      MaterialGPUChangeProj::Texture(key, stream) => {
        return if let Poll::Ready(r) = stream.poll_next(cx) {
          if let Some(r) = r {
            match r {
              GPUResourceChange::Content => Poll::Ready(Some(MaterialGPUChangeFlattened::Content)),
              GPUResourceChange::Reference => {
                Poll::Ready(Some(MaterialGPUChangeFlattened::ContentRef(*key)))
              }
            }
          } else {
            Poll::Ready(None)
          }
        } else {
          Poll::Pending
        }
      }
      MaterialGPUChangeProj::OwnedBindingContent => MaterialGPUChangeFlattened::Content,
      MaterialGPUChangeProj::OwnedBindingRef(key) => MaterialGPUChangeFlattened::ContentRef(*key),
      MaterialGPUChangeProj::ShaderHash => MaterialGPUChangeFlattened::ShaderHash,
    }))
  }
}

use __core::{
  pin::Pin,
  task::{Context, Poll},
};
use pin_project::pin_project;
#[pin_project]
struct MaterialGPUReactiveCell<T: WebGPUMaterialIncremental> {
  weak_source: SceneItemWeakRef<T>,
  gpu: T::GPU,
  #[pin]
  stream: T::Stream,
}

pub enum MaterialGPUChangeOutside {
  ShaderHash(u64),
  Binding,
}

impl<T: WebGPUMaterialIncremental> Stream for MaterialGPUReactiveCell<T> {
  type Item = MaterialGPUChangeOutside;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(r) = this.stream.poll_next(cx) {
      if let Some(delta) = r {
        if let Some(source) = self.weak_source.upgrade() {
          Poll::Ready(T::apply_change(delta))
        } else {
          Poll::Ready(None)
        }
      } else {
        Poll::Ready(None)
      }
    } else {
      Poll::Pending
    }
  }
}

pub trait WebGPUMaterialIncremental: Incremental {
  type GPU;
  type Stream: Stream;
  fn build_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResource,
  ) -> (Self::GPU, Self::Stream);
  fn apply_change(delta: <Self::Stream as Stream>::Item) -> Option<MaterialGPUChangeOutside>;

  fn build_gpu_cell(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResource,
  ) -> MaterialGPUReactiveCell<Self> {
    let (gpu, stream) = Self::build_gpu(source, ctx);

    MaterialGPUReactiveCell {
      weak_source: source.downgrade(),
      gpu,
      stream,
    }
  }
}

#[derive(Clone)]
pub struct TextureBuildCtxOwned {
  gpu: GPUDevice,
  mipmap_gen: Rc<RefCell<MipMapTaskManager>>,
}

// pub trait StreamBuilder {
//   type Stream;
//   fn build_forked(&self) -> Self::Stream;
// }

pub type ReactiveGPU2DTextureView =
  impl Stream<Item = GPUResourceChange> + Unpin + AsRef<GPU2DTextureView>;

pub fn create_texture2d_gpu_reactive(
  source: &SceneTexture2D,
  ctx: &TextureBuildCtx,
) -> ReactiveGPU2DTextureView {
  let texture = create_texture2d(source, ctx);
  source.listen_by(any_change).fold_signal(
    texture,
    move |change, texture: &mut GPU2DTextureView| {
      // *texture = create_texture2d(source, todo!());
      GPUResourceChange::Reference
    },
  )
}
