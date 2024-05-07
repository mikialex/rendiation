use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_geometry::OptionalNearest;
use rendiation_mesh_core::MeshBufferHitPoint;
use rendiation_mesh_generator::*;
use rendiation_scene_interaction::{SceneRayInteractive, SceneRayInteractiveCtx};
use webgpu::{default_dispatcher, FrameRenderPass, RenderComponent};

use super::WidgetDispatcher;
use crate::*;

pub struct AxisHelper {
  pub enabled: bool,
  pub root: SceneNode,
  x: Arrow,
  y: Arrow,
  z: Arrow,
}

impl PassContentWithSceneAndCamera for &mut AxisHelper {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    if !self.enabled {
      return;
    }

    // sort by the camera
    let center = scene.node_derives.get_world_matrix(&self.root).position();
    let camera_position = scene
      .node_derives
      .get_world_matrix(&camera.read().node)
      .position();
    let center_to_eye_dir = camera_position - center;
    let center_to_eye_dir = center_to_eye_dir.normalize();
    let x = Vec3::new(1., 0., 0.).dot(center_to_eye_dir);
    let y = Vec3::new(0., 1., 0.).dot(center_to_eye_dir);
    let z = Vec3::new(0., 0., 1.).dot(center_to_eye_dir);

    let mut arr = [(x, &mut self.x), (y, &mut self.y), (z, &mut self.z)];
    arr.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Less));

    arr
      .iter_mut()
      .for_each(|(_, a)| a.render(pass, scene, camera));
  }
}

type ArrowMaterial = FlatMaterial;

pub struct Arrow {
  cylinder: OverridableMeshModelImpl,
  tip: OverridableMeshModelImpl,
  material: IncrementalSignalPtr<ArrowMaterial>,
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
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponent,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    self.cylinder.render(pass, dispatcher, camera, scene);
    self.tip.render(pass, dispatcher, camera, scene);
  }
}

impl PassContentWithSceneAndCamera for Arrow {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    let dispatcher = &WidgetDispatcher::new(default_dispatcher(pass));
    SceneRenderable::render(self, pass, dispatcher, camera, scene);
  }
}

impl Arrow {
  pub fn new(parent: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let root = parent.create_child();

    let (cylinder_mesh, tip_mesh) = Arrow::default_shape();

    let material = solid_material((1., 1., 1.)).into_ptr();
    let modify_material = material.clone();
    let material = MaterialEnum::Flat(material);

    let node_cylinder = root.create_child();

    let model = StandardModel::new(material.clone(), cylinder_mesh);
    let model = ModelEnum::Standard(model.into());
    let model = SceneModelImpl::new(model, node_cylinder);
    let mut cylinder = model.into_matrix_overridable();
    cylinder.push_override(auto_scale.clone());

    let node_tip = root.create_child();
    node_tip.set_local_matrix(Mat4::translate((0., 2., 0.)));

    let model = StandardModel::new(material, tip_mesh);
    let model = ModelEnum::Standard(model.into());
    let model = SceneModelImpl::new(model, node_tip);
    let mut tip = model.into_matrix_overridable();
    tip.push_override(auto_scale.clone());

    Self {
      root,
      cylinder,
      tip,
      material: modify_material,
    }
  }

  pub fn default_shape() -> (MeshEnum, MeshEnum) {
    let config = TessellationConfig { u: 1, v: 10 };
    let cylinder_mesh = build_scene_mesh(|builder| {
      builder.triangulate_parametric(
        &CylinderMeshParameter {
          radius_top: 0.02,
          radius_bottom: 0.02,
          height: 2.,
          ..Default::default()
        }
        .body_surface(),
        config,
        true,
      );
    });

    let tip_mesh = build_scene_mesh(|builder| {
      builder.triangulate_parametric(
        &CylinderMeshParameter {
          radius_top: 0.0,
          radius_bottom: 0.06,
          height: 0.2,
          ..Default::default()
        }
        .body_surface(),
        config,
        true,
      );
    });

    (cylinder_mesh, tip_mesh)
  }

  pub fn set_color(&self, color: Vec3<f32>) {
    color
      .expand_with_one()
      .wrap(FlatMaterialDelta::color)
      .apply_modify(&self.material);
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
  FlatMaterial {
    color: color.into().expand_with_one(),
    // ext: DynamicExtension::default().with_insert(MaterialStates::helper_like()),
  }
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
