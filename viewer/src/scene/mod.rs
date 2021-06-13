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
pub mod texture_cube;
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
pub use texture_cube::*;
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

pub trait Material: MaterialStyleAbility<StandardForward> + 'static {}
impl<T> Material for T where T: MaterialStyleAbility<StandardForward> + 'static {}

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Box<dyn Background>,
  pub cameras: Arena<Camera>,
  pub lights: Arena<SceneLight>,
  pub models: Arena<Model>,
  pub meshes: Arena<SceneMesh>,
  pub materials: Arena<Box<dyn Material>>,
  pub samplers: WatchedArena<SceneSampler>,
  pub texture_2ds: WatchedArena<SceneTexture2D>,
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
      samplers: WatchedArena::new(),
      texture_2ds: WatchedArena::new(),
      pipeline_resource: PipelineResourceManager::new(),
      active_camera: None,
      active_camera_gpu: None,
      render_list: RenderList::new(),
    }
  }

  pub fn create_node(&mut self, builder: impl Fn(&mut SceneNode, &mut Self)) -> SceneNodeHandle {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    let new = self.nodes.create_node(node);
    let root = self.get_root_handle();
    self.nodes.node_add_child_by_id(root, new);
    new
  }

  pub fn background(&mut self, background: impl Background) -> &mut Self {
    self.background = Box::new(background);
    self
  }
}
