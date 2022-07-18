use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_renderable_mesh::tessellation::{CylinderMeshParameter, IndexedMeshTessellator};

use crate::*;

pub struct AxisHelper {
  pub enabled: bool,
  pub root: SceneNode,
  x: Arrow,
  y: Arrow,
  z: Arrow,
}

impl PassContentWithCamera for &mut AxisHelper {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    // sort by the camera
    let center = self.root.visit(|n| n.world_matrix.position());
    let camera_position = camera.node.visit(|n| n.world_matrix.position());
    let center_to_eye_dir = camera_position - center;
    let center_to_eye_dir = center_to_eye_dir.normalize();
    let x = Vec3::new(1., 0., 0.).dot(center_to_eye_dir);
    let y = Vec3::new(0., 1., 0.).dot(center_to_eye_dir);
    let z = Vec3::new(0., 0., 1.).dot(center_to_eye_dir);

    let mut arr = [(x, &mut self.x), (y, &mut self.y), (z, &mut self.z)];
    arr.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Less));

    arr.iter_mut().for_each(|(_, a)| a.render(pass, camera));
  }
}

pub struct Arrow {
  cylinder: Box<dyn SceneRenderable>,
  tip: Box<dyn SceneRenderable>,
  pub root: SceneNode,
}

impl SceneRenderable for Arrow {
  fn ray_pick_nearest(
    &self,
    world_ray: &rendiation_geometry::Ray3,
    conf: &rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig,
  ) -> Option<rendiation_geometry::Nearest<rendiation_renderable_mesh::mesh::MeshBufferHitPoint>>
  {
    // let result =  Nearest::none();
    // self.cylinder.ray_pick_nearest(world_ray, conf)
    None
  }

  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.cylinder.render(pass, dispatcher, camera);
    self.tip.render(pass, dispatcher, camera);
  }
}

impl PassContentWithCamera for Arrow {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    let dispatcher = &pass.default_dispatcher();
    SceneRenderable::render(self, pass, dispatcher, camera);
  }
}

impl Arrow {
  pub fn new_reused(
    parent: &SceneNode,
    auto_scale: &Rc<RefCell<ViewAutoScalable>>,
    material: &(impl WebGPUMaterial + Clone),
    cylinder_mesh: &(impl WebGPUMesh + Clone),
    tip_mesh: &(impl WebGPUMesh + Clone),
  ) -> Self {
    let root = parent.create_child();

    let node_cylinder = root.create_child();
    let mut cylinder = MeshModelImpl::new(
      material.clone().into_resourced(),
      cylinder_mesh.clone().into_resourced(),
      node_cylinder,
    )
    .into_matrix_overridable();

    cylinder.push_override(auto_scale.clone());

    let node_tip = root.create_child();
    node_tip.set_local_matrix(Mat4::translate(0., 1., 0.));
    let mut tip = MeshModelImpl::new(
      material.clone().into_resourced(),
      tip_mesh.clone().into_resourced(),
      node_tip,
    )
    .into_matrix_overridable();

    tip.push_override(auto_scale.clone());

    Self {
      root,
      cylinder: Box::new(cylinder),
      tip: Box::new(tip),
    }
  }

  pub fn default_shape() -> ((impl WebGPUMesh + Clone), (impl WebGPUMesh + Clone)) {
    let cylinder = CylinderMeshParameter {
      radius_top: 0.01,
      radius_bottom: 0.01,
      height: 2.,
      ..Default::default()
    }
    .tessellate();
    let cylinder = MeshCell::new(MeshSource::new(cylinder));

    let tip = CylinderMeshParameter {
      radius_top: 0.0,
      radius_bottom: 0.06,
      height: 0.2,
      ..Default::default()
    }
    .tessellate();
    let tip = MeshCell::new(MeshSource::new(tip));
    (cylinder, tip)
  }
}

pub fn solid_material(color: impl Into<Vec3<f32>>) -> impl WebGPUMaterial + Clone {
  let color = color.into();
  let mut material = FlatMaterial {
    color: Vec4::new(color.x, color.y, color.z, 1.0),
  }
  .use_state();
  material.states.depth_write_enabled = false;
  material.states.depth_compare = webgpu::CompareFunction::Always;
  material
}

impl AxisHelper {
  pub fn new(parent: &SceneNode) -> Self {
    let root = parent.create_child();

    let (cylinder, tip) = Arrow::default_shape();
    let (cylinder, tip) = (&cylinder, &tip);

    let auto_scale = &Rc::new(RefCell::new(ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    }));

    let x = Arrow::new_reused(
      &root,
      auto_scale,
      &solid_material((0.8, 0.1, 0.1)),
      cylinder,
      tip,
    );
    x.root.set_local_matrix(Mat4::rotate_z(-f32::PI() / 2.));

    let y = Arrow::new_reused(
      &root,
      auto_scale,
      &solid_material((0.1, 0.8, 0.1)),
      cylinder,
      tip,
    );
    y.root.set_local_matrix(Mat4::identity());
    // the cylinder is y up, so do nothing

    let z = Arrow::new_reused(
      &root,
      auto_scale,
      &solid_material((0.1, 0.1, 0.8)),
      cylinder,
      tip,
    );
    z.root.set_local_matrix(Mat4::rotate_x(f32::PI() / 2.));

    Self {
      root,
      enabled: true,
      x,
      y,
      z,
    }
  }
}
