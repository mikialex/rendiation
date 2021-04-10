use crate::SceneNode;
use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle, NextTraverseVisit};
use rendiation_texture::Sampler;

pub type SceneNodeHandle<T> = ArenaTreeNodeHandle<SceneNode<T>>;
pub type ModelHandle<T: SceneBackend> = Handle<T::Model>;
pub type MeshHandle<T: SceneBackend> = Handle<T::Mesh>;
pub type MaterialHandle<T: SceneBackend> = Handle<T::Material>;

pub trait SceneBackend {
  type Model;
  type Material;
  type Mesh;
  type Light;
  type Background;
}

pub struct Scene<T: SceneBackend> {
  pub nodes: ArenaTree<SceneNode<T>>,
  pub background: Option<Box<T::Background>>,
  pub models: Arena<T::Model>,
  pub meshes: Arena<T::Mesh>,
  pub materials: Arena<T::Material>,
  pub lights: Arena<T::Light>,
  pub samplers: Arena<Sampler>,
  // textures: Arena<Texture>,
  // buffers: Arena<Buffer>,
}

impl<T: SceneBackend> Scene<T> {
  pub fn new() -> Self {
    Self {
      nodes: ArenaTree::new(SceneNode::default()),
      background: None,
      models: Arena::new(),
      meshes: Arena::new(),
      lights: Arena::new(),
      materials: Arena::new(),
      samplers: Arena::new(),
    }
  }

  pub fn update(&mut self) {
    let root = self.get_root_handle();
    self.nodes.traverse(root, &mut Vec::new(), |this, parent| {
      let node_data = this.data_mut();
      node_data.update(parent.map(|p| p.data()));
      NextTraverseVisit::VisitChildren
    });
  }

  pub fn create_model(&mut self, creator: impl SceneModelCreator<T>) -> ModelHandle<T> {
    let model = creator.create_model(self);
    self.models.insert(model)
  }

  pub fn create_node(
    &mut self,
    builder: impl Fn(&mut SceneNode<T>, &mut Self),
  ) -> SceneNodeHandle<T> {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    self.nodes.create_node(node)
  }
}

pub trait SceneModelCreator<T: SceneBackend> {
  fn create_model(self, scene: &mut Scene<T>) -> T::Model;
}
