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

impl PassContent for AxisHelper {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut Scene,
    _resource: &mut ResourcePoolInner,
    pass_info: &PassTargetFormatInfo,
  ) {
    if !self.enabled {
      return;
    }

    if let Some(active_camera) = &mut scene.active_camera {
      let (active_camera, camera_gpu) = active_camera.get_updated_gpu(gpu);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass: pass_info,
        resources: &mut scene.resources,
      };

      self.x.update(gpu, &mut base);
      self.y.update(gpu, &mut base);
      self.z.update(gpu, &mut base);
    }
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut rendiation_webgpu::GPURenderPass<'a>,
    scene: &'a Scene,
    pass_info: &'a PassTargetFormatInfo,
  ) {
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

    arr
      .iter()
      .for_each(|(_, a)| a.setup_pass(pass, scene, pass_info));
  }
}

struct Arrow {
  cylinder: MeshModel,
  tip: MeshModel,
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

  fn setup_pass<'a>(
    &'a self,
    pass: &mut rendiation_webgpu::GPURenderPass<'a>,
    scene: &'a Scene,
    pass_info: &'a PassTargetFormatInfo,
  ) {
    self.cylinder.setup_pass(
      pass,
      scene.active_camera.as_ref().unwrap().expect_gpu(),
      &scene.resources,
      pass_info,
    );

    self.tip.setup_pass(
      pass,
      scene.active_camera.as_ref().unwrap().expect_gpu(),
      &scene.resources,
      pass_info,
    );
  }

  fn new(
    parent: &SceneNode,
    color: Vec3<f32>,
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
    let material = material(color);

    let root = parent.create_child();

    let node_cylinder = root.create_child();
    let cylinder = MeshModel::new(material.clone(), cylinder_mesh, node_cylinder);

    let node_tip = root.create_child();
    node_tip.mutate(|node| node.local_matrix = Mat4::translate(0., 1., 0.));
    let tip = MeshModel::new(material, tip_mesh, node_tip);

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
      height: 0.1,
      ..Default::default()
    }
    .tessellate();
    let tip = MeshCell::new(tip);

    let x = Arrow::new(&root, Vec3::new(1., 0., 0.), cylinder.clone(), tip.clone());
    x.root.mutate(|node| {
      node.local_matrix = Mat4::rotate_z(-f32::PI() / 2.);
    });

    let y = Arrow::new(&root, Vec3::new(0., 1., 0.), cylinder.clone(), tip.clone());
    y.root.mutate(|_| {
      // the cylinder is y up, so do nothing
    });

    let z = Arrow::new(&root, Vec3::new(0., 0., 1.), cylinder, tip);
    z.root.mutate(|node| {
      node.local_matrix = Mat4::rotate_x(f32::PI() / 2.);
    });

    Self {
      root,
      enabled: true,
      x,
      y,
      z,
    }
  }
}
