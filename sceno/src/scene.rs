use crate::{Background, Material, SceneNode, ShaderComponent, SolidBackground};
use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle, NextTraverseVisit};

pub trait SceneMesh {}

pub type MaterialHandle = Handle<Material>;
pub type MeshHandle = Handle<Box<dyn SceneMesh>>;
pub type ComponentHandle = Handle<Box<dyn ShaderComponent>>;
pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNode>;

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Box<dyn Background>,

  pub meshes: Arena<Box<dyn SceneMesh>>,
  pub materials: Arena<Material>,
  pub components: Arena<Box<dyn ShaderComponent>>,
  // samplers: Arena<Sampler>,
  // textures: Arena<Texture>,
  // buffers: Arena<Buffer>,
}

impl Scene {
  pub fn new() -> Self {
    Self {
      nodes: ArenaTree::new(SceneNode::default()),
      background: Box::new(SolidBackground::default()),
      meshes: Arena::new(),
      materials: Arena::new(),
      components: Arena::new(),
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
