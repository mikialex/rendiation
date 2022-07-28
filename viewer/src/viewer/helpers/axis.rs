use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_geometry::OptionalNearest;
use rendiation_renderable_mesh::{
  mesh::MeshBufferHitPoint,
  tessellation::{CylinderMeshParameter, IndexedMeshTessellator},
};

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
    let center = self.root.get_world_matrix().position();
    let camera_position = camera.read().node.get_world_matrix().position();
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

type ArrowMaterial = StateControl<FlatMaterial>;
type ArrowTipMesh = impl WebGPUMesh + Clone;
type ArrowBodyMesh = impl WebGPUMesh + Clone;

pub struct Arrow {
  cylinder: OverridableMeshModelImpl<ArrowBodyMesh, ArrowMaterial>,
  tip: OverridableMeshModelImpl<ArrowTipMesh, ArrowMaterial>,
  pub root: SceneNode,
}

impl SceneRayInteractive for Arrow {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self
      .cylinder
      .ray_pick_nearest(ctx)
      .or(self.tip.ray_pick_nearest(ctx))
  }
}

impl SceneRenderable for Arrow {
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
  pub fn new(parent: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let root = parent.create_child();

    let (cylinder_mesh, tip_mesh) = Arrow::default_shape();
    let material = solid_material((1., 1., 1.));

    let node_cylinder = root.create_child();
    node_cylinder.set_local_matrix(Mat4::translate((0., 1., 0.)));
    let mut cylinder =
      MeshModelImpl::new(material.clone(), cylinder_mesh, node_cylinder).into_matrix_overridable();

    cylinder.push_override(auto_scale.clone());

    let node_tip = root.create_child();
    node_tip.set_local_matrix(Mat4::translate((0., 2., 0.)));
    let mut tip = MeshModelImpl::new(material, tip_mesh, node_tip).into_matrix_overridable();

    tip.push_override(auto_scale.clone());

    Self {
      root,
      cylinder,
      tip,
    }
  }

  pub fn default_shape() -> (ArrowBodyMesh, ArrowTipMesh) {
    let cylinder = CylinderMeshParameter {
      radius_top: 0.01,
      radius_bottom: 0.01,
      height: 2.,
      ..Default::default()
    }
    .tessellate();
    let cylinder = MeshSource::new(cylinder);

    let tip = CylinderMeshParameter {
      radius_top: 0.0,
      radius_bottom: 0.06,
      height: 0.2,
      ..Default::default()
    }
    .tessellate();
    let tip = MeshSource::new(tip);
    (cylinder.into_ref(), tip.into_ref())
  }

  pub fn set_color(&self, color: Vec3<f32>) {
    self.tip.material.write().material.color = (color.x, color.y, color.z, 1.).into();
    self.cylinder.material.write().material.color = (color.x, color.y, color.z, 1.).into();
  }

  pub fn with_transform(self, m: Mat4<f32>) -> Self {
    self.root.set_local_matrix(m);
    self
  }
  pub fn toward_x(self) -> Self {
    self.with_transform(Mat4::rotate_z(-f32::PI() / 2.))
  }
  pub fn toward_y(self) -> Self {
    // the cylinder is y up by default, so do nothing
    self
  }
  pub fn toward_z(self) -> Self {
    self.with_transform(Mat4::rotate_x(f32::PI() / 2.))
  }
}

pub fn solid_material(color: impl Into<Vec3<f32>>) -> ArrowMaterial {
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

    let auto_scale = &Rc::new(RefCell::new(ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    }));

    let x = Arrow::new(&root, auto_scale).toward_x();
    let y = Arrow::new(&root, auto_scale).toward_y();
    let z = Arrow::new(&root, auto_scale).toward_z();

    Self {
      root,
      enabled: true,
      x,
      y,
      z,
    }
  }
}
