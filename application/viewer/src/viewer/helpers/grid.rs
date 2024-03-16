use rendiation_algebra::*;
use rendiation_mesh_core::{GroupedMesh, NoneIndexedMesh};

use super::*;
use crate::*;

pub struct GridHelper {
  pub enabled: bool,
  pub root: SceneNode,
  pub config: GridConfig,
  mesh: HelperLineModel,
}

impl GridHelper {
  pub fn new(root: &SceneNode, config: GridConfig) -> Self {
    let mesh = build_grid(&config);
    let mat = WidenedLineMaterial::new(1.);
    let root = root.clone();
    let node = root.create_child();
    let mesh = HelperLineModel::new(mat, mesh, &node);

    Self {
      enabled: true,
      root,
      config,
      mesh,
    }
  }
}

impl PassContentWithSceneAndCamera for &mut GridHelper {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    if !self.enabled {
      return;
    }

    self.mesh.inner.render(
      pass,
      &WidgetDispatcher::new(default_dispatcher(pass)),
      camera,
      scene,
    )
  }
}

pub struct GridConfig {
  pub width_segments: usize,
  pub height_segments: usize,
  pub width: f32,
  pub height: f32,
}

impl Default for GridConfig {
  fn default() -> Self {
    Self {
      width_segments: 10,
      height_segments: 10,
      width: 1.,
      height: 1.,
    }
  }
}

fn build_grid(config: &GridConfig) -> HelperLineMesh {
  let mut lines = Vec::new();

  let color = Vec4::new(0., 0., 0., 0.5);

  for x in 0..=config.width_segments {
    let start = Vec3::new(x as f32 * config.width, 0., 0 as f32);
    let end = Vec3::new(x as f32, 0., config.height_segments as f32 * config.height);

    let line = WidenedLineVertex { start, end, color };
    lines.push(line)
  }

  for y in 0..=config.height_segments {
    let start = Vec3::new(0 as f32, 0., y as f32 * config.height);
    let end = Vec3::new(config.width_segments as f32 * config.width, 0., y as f32);

    let line = WidenedLineVertex { start, end, color };
    lines.push(line)
  }

  let lines = NoneIndexedMesh::new(lines);
  let lines = GroupedMesh::full(lines);
  HelperLineMesh::new(lines)
}
