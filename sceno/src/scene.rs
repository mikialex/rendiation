use crate::{Material, SceneMesh, SceneNode};
use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle, NextTraverseVisit};
use rendiation_texture::Sampler;

pub type MaterialHandle = Handle<Box<dyn Material>>;
pub type MeshHandle = Handle<Box<dyn SceneMesh>>;
pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNode>;

pub trait SceneBackend {
  type Drawable;
  type Material;
  type Mesh;
  type Light;
  type Background;
}

pub struct Scene<T: SceneBackend> {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Option<Box<T::Background>>,

  pub drawables: Arena<T::Drawable>,
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
      drawables: Arena::new(),
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
}
