use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_renderable_mesh::tessellation::{CylinderMeshParameter, IndexedMeshTessellator};
use rendiation_webgpu::*;

use crate::*;

pub struct AxisHelper {
  pub enabled: bool,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  x: Arrow,
  y: Arrow,
  z: Arrow,
}

impl PassContent for AxisHelper {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx) {
    if !self.enabled {
      return;
    }

    let mut base = scene.create_material_ctx_base(gpu, ctx.pass_info, &DefaultPassDispatcher);

    let root_position = self.root.visit(|n| n.world_matrix.position());

    self.auto_scale.borrow_mut().override_position = root_position.into();

    self.x.update(gpu, &mut base);
    self.y.update(gpu, &mut base);
    self.z.update(gpu, &mut base);
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    if !self.enabled {
      return;
    }
    let center = self.root.visit(|n| n.world_matrix.position());
    let camera = scene.active_camera.as_ref().unwrap();
    let camera = camera.node.visit(|n| n.world_matrix.position());
    let center_to_eye_dir = camera - center;
    let center_to_eye_dir = center_to_eye_dir.normalize();
    let x = Vec3::new(1., 0., 0.).dot(center_to_eye_dir);
    let y = Vec3::new(0., 1., 0.).dot(center_to_eye_dir);
    let z = Vec3::new(0., 0., 1.).dot(center_to_eye_dir);

    let mut arr = [(x, &self.x), (y, &self.y), (z, &self.z)];
    arr.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Less));

    arr.iter().for_each(|(_, a)| a.setup_pass(pass, scene));
  }
}

struct Arrow {
  cylinder: OverridableMeshModelImpl,
  tip: OverridableMeshModelImpl,
  root: SceneNode,
}

impl Arrow {
  pub fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    ctx: &mut SceneMaterialRenderPrepareCtxBase,
  ) {
    self.cylinder.update(gpu, ctx);
    self.tip.update(gpu, ctx);
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    let camera = scene
      .resources
      .cameras
      .expect_gpu(scene.active_camera.as_ref().unwrap());

    self.cylinder.setup_pass(pass, camera, &scene.resources);
    self.tip.setup_pass(pass, camera, &scene.resources);
  }

  fn new(
    parent: &SceneNode,
    auto_scale: Rc<RefCell<ViewAutoScalable>>,
    color: impl Into<Vec3<f32>>,
    cylinder_mesh: impl Mesh + 'static,
    tip_mesh: impl Mesh + 'static,
  ) -> Self {
    fn material(color: Vec3<f32>) -> impl Material + Clone {
      let mut material = FlatMaterial {
        color: Vec4::new(color.x, color.y, color.z, 1.0),
      }
      .into_scene_material();
      material.states.depth_write_enabled = false;
      material.states.depth_compare = wgpu::CompareFunction::Always;
      MaterialCell::new(material)
    }
    let material = material(color.into());

    let root = parent.create_child();

    let node_cylinder = root.create_child();
    let mut cylinder =
      MeshModelImpl::new(material.clone(), cylinder_mesh, node_cylinder).into_matrix_overridable();

    cylinder.push_override(auto_scale.clone());

    let node_tip = root.create_child();
    node_tip.mutate(|node| node.local_matrix = Mat4::translate(0., 1., 0.));
    let mut tip = MeshModelImpl::new(material, tip_mesh, node_tip).into_matrix_overridable();

    tip.push_override(auto_scale);

    Self {
      root,
      cylinder,
      tip,
    }
  }
}

impl AxisHelper {
  pub fn new(parent: &SceneNode) -> Self {
    let root = parent.create_child();

    let cylinder = CylinderMeshParameter {
      radius_top: 0.01,
      radius_bottom: 0.01,
      height: 2.,
      ..Default::default()
    }
    .tessellate();
    let cylinder = MeshCell::new(cylinder);

    let tip = CylinderMeshParameter {
      radius_top: 0.0,
      radius_bottom: 0.06,
      height: 0.2,
      ..Default::default()
    }
    .tessellate();
    let tip = MeshCell::new(tip);

    let auto_scale = Rc::new(RefCell::new(ViewAutoScalable {
      override_position: None,
      independent_scale_factor: 100.,
    }));

    let x = Arrow::new(
      &root,
      auto_scale.clone(),
      (0.8, 0.1, 0.1),
      cylinder.clone(),
      tip.clone(),
    );
    x.root.mutate(|node| {
      node.local_matrix = Mat4::rotate_z(-f32::PI() / 2.);
    });

    let y = Arrow::new(
      &root,
      auto_scale.clone(),
      (0.1, 0.8, 0.1),
      cylinder.clone(),
      tip.clone(),
    );
    y.root.mutate(|_| {
      // the cylinder is y up, so do nothing
    });

    let z = Arrow::new(&root, auto_scale.clone(), (0.1, 0.1, 0.8), cylinder, tip);
    z.root.mutate(|node| {
      node.local_matrix = Mat4::rotate_x(f32::PI() / 2.);
    });

    Self {
      root,
      enabled: true,
      auto_scale,
      x,
      y,
      z,
    }
  }
}
