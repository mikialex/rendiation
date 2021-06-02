pub mod background;
pub mod camera;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod sampler;
pub mod texture;
pub mod util;

pub use background::*;
pub use camera::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
pub use sampler::*;
pub use texture::*;
pub use util::*;

pub use arena::*;
pub use arena_tree::*;

use crate::renderer::*;

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNode>;
pub type ModelHandle = Handle<Model>;
pub type MeshHandle = Handle<SceneMesh>;
pub type MaterialHandle = Handle<Box<dyn Material>>;
pub type LightHandle = Handle<Box<dyn Light>>;
pub type SamplerHandle = Handle<SceneSampler>;
pub type Texture2DHandle = Handle<SceneTexture2D>;

pub trait Material: MaterialStyleAbility<StandardForward> {}
impl<T> Material for T where T: MaterialStyleAbility<StandardForward> {}

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Box<dyn Background>,
  pub cameras: Arena<Camera>,
  pub lights: Arena<SceneLight>,
  pub models: Arena<Model>,
  pub meshes: Arena<SceneMesh>,
  pub materials: Arena<Box<dyn Material>>,
  pub samplers: Arena<SceneSampler>,
  pub texture_2ds: Arena<SceneTexture2D>,
  // buffers: Arena<Buffer>,
  pub(crate) pipeline_resource: PipelineResourceManager,
  pub active_camera: Option<Camera>,
  pub active_camera_gpu: Option<CameraBindgroup>,
  pub render_list: RenderList,
}

impl Scene {
  pub fn new() -> Self {
    Self {
      nodes: ArenaTree::new(SceneNode::default()),
      background: Box::new(SolidBackground::default()),
      cameras: Arena::new(),
      models: Arena::new(),
      meshes: Arena::new(),
      lights: Arena::new(),
      materials: Arena::new(),
      samplers: Arena::new(),
      texture_2ds: Arena::new(),
      pipeline_resource: PipelineResourceManager::new(),
      active_camera: None,
      active_camera_gpu: None,
      render_list: RenderList::new(),
    }
  }

  // pub fn create_model(&mut self, creator: impl SceneModelCreator) -> ModelHandle {
  //   creator.create_model(self)
  // }

  // pub fn create_light(&mut self, creator: impl SceneLightCreator) -> LightHandle {
  //   creator.create_light(self)
  // }

  pub fn create_node(&mut self, builder: impl Fn(&mut SceneNode, &mut Self)) -> &mut Self {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    let new = self.nodes.create_node(node);
    let root = self.get_root_handle();
    self.nodes.node_add_child_by_id(root, new);
    self
  }

  // pub fn model_node(&mut self, model: impl SceneModelCreator) -> &mut Self {
  //   let model = self.create_model(model);
  //   self.create_node(|node, _| node.payloads.push(SceneNodePayload::Model(model)));
  //   self
  // }

  // pub fn model_node_with_modify(
  //   &mut self,
  //   model: impl SceneModelCreator,
  //   m: impl Fn(&mut SceneNode),
  // ) -> &mut Self {
  //   let model = self.create_model(model);
  //   self.create_node(|node, _| {
  //     node.payloads.push(SceneNodePayload::Model(model));
  //     m(node)
  //   });
  //   self
  // }

  pub fn background(&mut self, background: impl Background) -> &mut Self {
    self.background = Box::new(background);
    self
  }
}

// pub trait SceneModelCreator<T: SceneBackend> {
//   fn create_model(self, scene: &mut Scene) -> ModelHandle;
// }

// impl SceneModelCreator for <T as SceneBackend>::Model
// where
//   T: SceneBackend,
// {
//   fn create_model(self, scene: &mut Scene) -> ModelHandle {
//     scene.models.insert(self)
//   }
// }

// pub trait SceneLightCreator<T: SceneBackend> {
//   fn create_light(self, scene: &mut Scene) -> LightHandle;
// }

// impl SceneLightCreator for <T as SceneBackend>::Light
// where
//   T: SceneBackend,
// {
//   fn create_light(self, scene: &mut Scene) -> LightHandle {
//     scene.lights.insert(self)
//   }
// }
