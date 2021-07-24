pub mod background;
pub mod bindgroup;
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

use std::collections::HashSet;

pub use background::*;
pub use bindgroup::*;
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

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNode>;
pub type ModelHandle = Handle<Box<dyn Model>>;
pub type MeshHandle = Handle<Box<dyn Mesh>>;
pub type MaterialHandle = Handle<Box<dyn Material>>;
pub type LightHandle = Handle<Box<dyn Light>>;
pub type SamplerHandle = Handle<SceneSampler>;
pub type Texture2DHandle = Handle<SceneTexture2D>;

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Box<dyn Background>,
  pub cameras: Arena<Camera>,
  pub lights: Arena<SceneLight>,
  pub models: Arena<Box<dyn Model>>,
  pub meshes: Arena<Box<dyn Mesh>>,
  pub materials: Arena<Box<dyn Material>>,
  pub samplers: WatchedArena<SceneSampler>,
  pub texture_2ds: WatchedArena<SceneTexture2D>,
  pub(crate) pipeline_resource: PipelineResourceManager,
  pub active_camera: Option<Camera>,
  pub active_camera_gpu: Option<CameraBindgroup>,
  pub render_list: RenderList,
  pub reference_finalization: ReferenceFinalization,
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
      reference_finalization: Default::default(),
    }
  }

  pub fn maintain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    let mut material_change = HashSet::new();
    self.samplers.drain_modified().for_each(|(sampler, _)| {
      sampler.update(device, queue);
      sampler.foreach_material_refed(|handle| {
        material_change.insert(handle);
      });
    });
    self.texture_2ds.drain_modified().for_each(|(tex, _)| {
      tex.update(device, queue);
      tex.foreach_material_refed(|handle| {
        material_change.insert(handle);
      });
    });
    material_change
      .drain()
      .for_each(|h| self.materials.get_mut(h).unwrap().on_ref_resource_changed());

    self
      .reference_finalization
      .maintain(&self.samplers, &self.texture_2ds);
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
