pub mod background;
pub mod bindgroup;
pub mod camera;
pub mod fatline;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod rendering;
pub mod texture;
pub mod texture_cube;

pub mod util;

use std::{
  collections::{HashMap, HashSet},
  rc::Rc,
};

pub use background::*;
pub use bindgroup::*;
pub use camera::*;
pub use fatline::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use rendering::*;
use rendiation_texture::TextureSampler;
use rendiation_webgpu::GPU;
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
pub type Texture2DHandle = Handle<SceneTexture2D>;
pub type TextureCubeHandle = Handle<SceneTextureCube>;

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Box<dyn Background>,
  pub cameras: Arena<Camera>,
  pub lights: Arena<SceneLight>,
  pub models: Arena<Box<dyn Model>>,
  pub meshes: Arena<Box<dyn Mesh>>,
  pub materials: Arena<Box<dyn Material>>,
  pub samplers: HashMap<TextureSampler, Rc<wgpu::Sampler>>,
  pub texture_2ds: WatchedArena<SceneTexture2D>,
  pub texture_cubes: WatchedArena<SceneTextureCube>,
  pub(crate) pipeline_resource: PipelineResourceManager,
  pub(crate) layouts: BindGroupLayoutManager,
  pub active_camera: Option<Camera>,
  pub reference_finalization: ReferenceFinalization,

  has_registered: bool, // todo improve
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
      samplers: HashMap::new(),
      texture_2ds: WatchedArena::new(),
      texture_cubes: WatchedArena::new(),
      pipeline_resource: PipelineResourceManager::new(),
      layouts: BindGroupLayoutManager::new(),
      active_camera: None,
      reference_finalization: Default::default(),
      has_registered: false,
    }
  }

  pub fn register_layout(&mut self, device: &wgpu::Device) {
    if self.has_registered {
      return;
    }
    self
      .layouts
      .register::<TransformGPU>(TransformGPU::layout(device));
    self
      .layouts
      .register::<CameraBindgroup>(CameraBindgroup::layout(device));
    self.has_registered = true;
  }

  pub fn maintain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
    self.register_layout(device);

    let root = self.get_root_handle();
    self
      .nodes
      .traverse_mut(root, &mut Vec::new(), |this, parent| {
        let node_data = this.data_mut();
        node_data.hierarchy_update(parent.map(|p| p.data()));
        if node_data.net_visible {
          NextTraverseVisit::SkipChildren
        } else {
          NextTraverseVisit::VisitChildren
        }
      });

    let mut material_bindgroup_dirtied = HashSet::new();
    self.texture_2ds.drain_modified().for_each(|(tex, _)| {
      tex.update(device, queue);
      tex.foreach_material_refed(|handle| {
        material_bindgroup_dirtied.insert(handle);
      });
    });

    self.texture_cubes.drain_modified().for_each(|(tex, _)| {
      tex.update(device, queue);
      tex.foreach_material_refed(|handle| {
        material_bindgroup_dirtied.insert(handle);
      });
    });

    material_bindgroup_dirtied
      .drain()
      .for_each(|h| self.materials.get_mut(h).unwrap().on_ref_resource_changed());

    self
      .reference_finalization
      .maintain(&self.texture_2ds, &self.texture_cubes);
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

impl Default for Scene {
  fn default() -> Self {
    Self::new()
  }
}

pub trait SceneRenderable {
  fn update(
    &mut self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtxBase,
    materials: &mut Arena<Box<dyn Material>>,
    meshes: &mut Arena<Box<dyn Mesh>>,
    nodes: &mut ArenaTree<SceneNode>,
  );

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    materials: &'a Arena<Box<dyn Material>>,
    meshes: &'a Arena<Box<dyn Mesh>>,
    nodes: &'a ArenaTree<SceneNode>,
    camera_gpu: &'a CameraBindgroup,
    pipeline_resource: &'a PipelineResourceManager,
    pass_info: &'a PassTargetFormatInfo,
  );
}
