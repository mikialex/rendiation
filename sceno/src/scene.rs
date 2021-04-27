use crate::{SceneNode, SceneNodePayload};
use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle, NextTraverseVisit};
use rendiation_texture::Sampler;

pub type SceneNodeHandle<T> = ArenaTreeNodeHandle<SceneNode<T>>;
pub type ModelHandle<T> = Handle<<T as SceneBackend>::Model>;
pub type MeshHandle<T> = Handle<<T as SceneBackend>::Mesh>;
pub type MaterialHandle<T> = Handle<<T as SceneBackend>::Material>;
pub type LightHandle<T> = Handle<<T as SceneBackend>::Light>;

pub trait SceneBackend {
  type Model;
  type Material;
  type Mesh;
  type Light;
  type Background;
}

pub struct Scene<T: SceneBackend> {
  pub nodes: ArenaTree<SceneNode<T>>,
  pub background: Option<T::Background>,
  pub lights: Arena<T::Light>,
  pub models: Arena<T::Model>,
  pub meshes: Arena<T::Mesh>,
  pub materials: Arena<T::Material>,
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
    creator.create_model(self)
  }

  pub fn create_light(&mut self, creator: impl SceneLightCreator<T>) -> LightHandle<T> {
    creator.create_light(self)
  }

  pub fn create_node(&mut self, builder: impl Fn(&mut SceneNode<T>, &mut Self)) -> &mut Self {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    let new = self.nodes.create_node(node);
    let root = self.get_root_handle();
    self.nodes.node_add_child_by_id(root, new);
    self
  }

  pub fn model_node(&mut self, model: impl SceneModelCreator<T>) -> &mut Self {
    let model = self.create_model(model);
    self.create_node(|node, _| node.payload.push(SceneNodePayload::Model(model)));
    self
  }

  pub fn model_node_with_modify(
    &mut self,
    model: impl SceneModelCreator<T>,
    m: impl Fn(&mut SceneNode<T>),
  ) -> &mut Self {
    let model = self.create_model(model);
    self.create_node(|node, _| {
      node.payload.push(SceneNodePayload::Model(model));
      m(node)
    });
    self
  }

  pub fn background(&mut self, background: T::Background) -> &mut Self {
    self.background = background.into();
    self
  }
}

pub trait SceneModelCreator<T: SceneBackend> {
  fn create_model(self, scene: &mut Scene<T>) -> ModelHandle<T>;
}

impl<T> SceneModelCreator<T> for <T as SceneBackend>::Model
where
  T: SceneBackend,
{
  fn create_model(self, scene: &mut Scene<T>) -> ModelHandle<T> {
    scene.models.insert(self)
  }
}

pub trait SceneLightCreator<T: SceneBackend> {
  fn create_light(self, scene: &mut Scene<T>) -> LightHandle<T>;
}

impl<T> SceneLightCreator<T> for <T as SceneBackend>::Light
where
  T: SceneBackend,
{
  fn create_light(self, scene: &mut Scene<T>) -> LightHandle<T> {
    scene.lights.insert(self)
  }
}
