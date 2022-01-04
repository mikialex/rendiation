use std::{cell::RefCell, rc::Rc};

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};
use rendiation_algebra::PerspectiveProjection;
use rendiation_webgpu::{RenderPassInfo, GPU};

use crate::*;

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
    self.resources.maintain();
  }

  pub fn create_material_ctx_base<'a>(
    &'a mut self,
    gpu: &GPU,
    pass_info: &'a RenderPassInfo,
    pass: &'a dyn PassDispatcher,
  ) -> SceneMaterialRenderPrepareCtxBase<'a> {
    let camera = self
      .active_camera
      .as_mut()
      .unwrap_or(&mut self.default_camera);
    self.resources.cameras.check_update_gpu(camera, gpu);

    SceneMaterialRenderPrepareCtxBase {
      camera,
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
