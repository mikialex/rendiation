use std::{cell::RefCell, rc::Rc, sync::Arc};

use incremental::clone_self_incremental;
use rendiation_algebra::*;
use rendiation_geometry::OptionalNearest;
use rendiation_mesh_generator::*;
use rendiation_renderable_mesh::{mesh::MeshBufferHitPoint, vertex::Vertex, TriangleList};

use crate::*;

use super::WidgetDispatcher;

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
type ArrowTipMesh = impl WebGPUSceneMesh;
type ArrowBodyMesh = impl WebGPUSceneMesh;

pub struct Arrow {
  cylinder: OverridableMeshModelImpl,
  tip: OverridableMeshModelImpl,
  material: SceneItemRef<ArrowMaterial>,
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
    let dispatcher = &WidgetDispatcher::new(pass.default_dispatcher());
    SceneRenderable::render(self, pass, dispatcher, camera);
  }
}

impl Arrow {
  pub fn new(parent: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let root = parent.create_child();

    let (cylinder_mesh, tip_mesh) = Arrow::default_shape();

    let cylinder_mesh: Box<dyn WebGPUSceneMesh> = Box::new(cylinder_mesh);
    let cylinder_mesh = SceneMeshType::Foreign(Arc::new(cylinder_mesh));

    let tip_mesh: Box<dyn WebGPUSceneMesh> = Box::new(tip_mesh);
    let tip_mesh = SceneMeshType::Foreign(Arc::new(tip_mesh));

    let material = solid_material((1., 1., 1.));
    let material = SceneItemRef::new(material);
    let modify_material = material.clone();
    let material: Box<dyn WebGPUSceneMaterial> = Box::new(material);
    let material = SceneMaterialType::Foreign(Arc::new(material)).into_ref();

    let node_cylinder = root.create_child();

    let model = StandardModel {
      material: material.clone(),
      mesh: cylinder_mesh.into(),
      group: Default::default(),
    };
    let model = SceneModelType::Standard(model.into());
    let model = SceneModelImpl {
      model,
      node: node_cylinder,
    };
    let mut cylinder = model.into_matrix_overridable();
    cylinder.push_override(auto_scale.clone());

    let node_tip = root.create_child();
    node_tip.set_local_matrix(Mat4::translate((0., 2., 0.)));

    let model = StandardModel {
      material,
      mesh: tip_mesh.into(),
      group: Default::default(),
    };
    let model = SceneModelType::Standard(model.into());
    let model = SceneModelImpl {
      model,
      node: node_tip,
    };
    let mut tip = model.into_matrix_overridable();
    tip.push_override(auto_scale.clone());

    Self {
      root,
      cylinder,
      tip,
      material: modify_material,
    }
  }

  pub fn default_shape() -> (ArrowBodyMesh, ArrowTipMesh) {
    let config = TessellationConfig { u: 1, v: 10 };
    let cylinder = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
      .triangulate_parametric(
        &CylinderMeshParameter {
          radius_top: 0.02,
          radius_bottom: 0.02,
          height: 2.,
          ..Default::default()
        }
        .body_surface(),
        config,
        true,
      )
      .build_mesh_into();

    let cylinder = MeshSource::new(cylinder);

    let tip = IndexedMeshBuilder::<TriangleList, Vec<Vertex>>::default()
      .triangulate_parametric(
        &CylinderMeshParameter {
          radius_top: 0.0,
          radius_bottom: 0.06,
          height: 0.2,
          ..Default::default()
        }
        .body_surface(),
        config,
        true,
      )
      .build_mesh_into();
    let tip = MeshSource::new(tip);
    (cylinder.into_ref(), tip.into_ref())
  }

  pub fn set_color(&self, color: Vec3<f32>) {
    self.material.write().material.color = (color.x, color.y, color.z, 1.).into();
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
  FlatMaterial {
    color: Vec4::new(color.x, color.y, color.z, 1.0),
  }
  .use_state_helper_like()
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
