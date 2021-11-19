pub mod background;
pub mod bindgroup;
pub mod camera;
pub mod fatline;
pub mod lights;
pub mod materials;
pub mod mesh;
pub mod model;
pub mod node;
pub mod picking;
pub mod rendering;
pub mod texture;
pub mod util;

use std::{cell::RefCell, rc::Rc};

pub use anymap::AnyMap;
pub use background::*;
pub use bindgroup::*;
pub use camera::*;
pub use fatline::*;
pub use lights::*;
pub use materials::*;
pub use mesh::*;
pub use model::*;
pub use node::*;
pub use picking::*;
pub use rendering::*;
use rendiation_texture::TextureSampler;
pub use texture::*;
pub use util::*;

pub use arena::*;
pub use arena_tree::*;

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};

use rendiation_webgpu::{
  BindGroupLayoutCache, GPURenderPass, PipelineResourceCache, SamplerCache, GPU,
};

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNodeData>;
pub type LightHandle = Handle<Box<dyn Light>>;

pub struct Scene {
  pub background: Box<dyn Background>,

  pub active_camera: Option<SceneCamera>,
  pub cameras: Arena<SceneCamera>,
  pub lights: Arena<SceneLight>,
  pub models: Vec<MeshModel>,

  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  pub root: SceneNode,
  pub resources: GPUResourceCache,
}

impl Scene {
  pub fn new() -> Self {
    let nodes: Rc<RefCell<ArenaTree<SceneNodeData>>> = Default::default();

    let root = SceneNode::from_root(nodes.clone());

    Self {
      nodes,
      root,
      background: Box::new(SolidBackground::default()),
      cameras: Arena::new(),
      lights: Arena::new(),
      models: Vec::new(),

      active_camera: None,
      resources: Default::default(),
    }
  }

  pub fn maintain(&mut self, gpu: &GPU) {
    let mut nodes = self.nodes.borrow_mut();
    let root = nodes.root();
    nodes.traverse_mut(root, &mut Vec::new(), |this, parent| {
      let node_data = this.data_mut();
      node_data.hierarchy_update(gpu, parent.map(|p| p.data()));
      if node_data.net_visible {
        NextTraverseVisit::VisitChildren
      } else {
        NextTraverseVisit::SkipChildren
      }
    });
  }
}

impl Default for Scene {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Default)]
pub struct SceneComponents {
  pub nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
}

pub trait SceneRenderable {
  fn update(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtxBase);

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  );
}

/// GPU cache container for given scene
///
/// Resources once allocate never release until the cache drop
pub struct GPUResourceCache {
  pub(crate) samplers: SamplerCache<TextureSampler>,
  pub(crate) pipeline_resource: PipelineResourceCache,
  pub(crate) layouts: BindGroupLayoutCache,
  pub(crate) custom_storage: AnyMap,
}

impl Default for GPUResourceCache {
  fn default() -> Self {
    Self {
      samplers: Default::default(),
      pipeline_resource: Default::default(),
      layouts: Default::default(),
      custom_storage: AnyMap::new(),
    }
  }
}
