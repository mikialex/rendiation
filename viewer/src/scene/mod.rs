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
use rendiation_algebra::PerspectiveProjection;
use rendiation_texture::TextureSampler;
pub use texture::*;
pub use util::*;

pub use arena::*;
pub use arena_tree::*;

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};

use rendiation_webgpu::{
  BindGroupLayoutCache, PipelineResourceCache, RenderPassInfo, SamplerCache, GPU,
};

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNodeData>;
pub type LightHandle = Handle<Box<dyn Light>>;

pub struct Scene {
  pub background: Box<dyn Background>,

  pub default_camera: SceneCamera,
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

    let default_camera = PerspectiveProjection::default();
    let camera_node = root.create_child();
    let default_camera = SceneCamera::new(default_camera, camera_node);

    Self {
      nodes,
      root,
      background: Box::new(SolidBackground::default()),
      default_camera,
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

  pub fn create_material_ctx_base<'a>(
    &'a mut self,
    gpu: &GPU,
    pass_info: &'a RenderPassInfo,
    pass: &'a dyn PassDispatcher,
  ) -> SceneMaterialRenderPrepareCtxBase<'a> {
    let active_camera = self
      .active_camera
      .as_mut()
      .unwrap_or(&mut self.default_camera);
    let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu);

    SceneMaterialRenderPrepareCtxBase {
      active_camera,
      camera_gpu,
      pass_info,
      resources: &mut self.resources,
      pass,
    }
  }
}

impl Default for Scene {
  fn default() -> Self {
    Self::new()
  }
}

pub trait SceneRenderable {
  fn update(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtxBase);

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
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
