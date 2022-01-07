use rendiation_algebra::*;
use rendiation_webgpu::GPU;

use crate::*;

use super::HelperLineMesh;

pub struct GridHelper {
  pub enabled: bool,
  pub root: SceneNode,
  pub config: GridConfig,
  mesh: FatlineImpl,
}

impl GridHelper {
  pub fn new(root: &SceneNode, config: GridConfig) -> Self {
    let mesh = build_grid(&config);
    let mesh = FatlineMeshCellImpl::from(mesh);
    let mat = FatLineMaterial {
      width: 1.,
      states: Default::default(),
    }
    .into_scene_material();
    let mat = MaterialCell::new(mat);
    let root = root.clone();
    let node = root.create_child();
    let mesh = FatlineImpl::new_typed(mat, mesh, node);

    Self {
      enabled: true,
      root,
      config,
      mesh,
    }
  }
}

impl PassContent for GridHelper {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    if !self.enabled {
      return;
    }

    let mut base = scene.create_material_ctx_base(gpu, ctx.pass_info, &DefaultPassDispatcher);
    self.mesh.update(gpu, &mut base);
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    if !self.enabled {
      return;
    }

    let camera = scene
      .resources
      .cameras
      .expect_gpu(scene.active_camera.as_ref().unwrap());

    self.mesh.setup_pass(pass, camera, &scene.resources);
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

    let line = FatLineVertex { start, end, color };
    lines.push(line)
  }

  for y in 0..=config.height_segments {
    let start = Vec3::new(0 as f32, 0., y as f32 * config.height);
    let end = Vec3::new(config.width_segments as f32 * config.width, 0., y as f32);

    let line = FatLineVertex { start, end, color };
    lines.push(line)
  }

  HelperLineMesh::new(lines)
}
