use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_webgpu::GPU;

use crate::*;

impl SceneRenderable for MeshModel {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, base)
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let inner = self.inner.borrow();
    inner.setup_pass(pass, camera_gpu, resources)
  }
}

impl MeshModelImpl {
  pub fn into_matrix_overridable(self) -> OverridableMeshModelImpl {
    OverridableMeshModelImpl {
      inner: self,
      override_gpu: None,
      overrides: Vec::with_capacity(1),
    }
  }
}

impl SceneRenderable for MeshModelImpl {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let material = &mut self.material;
    let mesh = &mut self.mesh;

    self.node.check_update_gpu(base.resources, gpu);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      active_mesh: mesh.as_ref().into(),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    self.node.visit(|node| {
      let ctx = SceneMaterialPassSetupCtx {
        camera_gpu,
        model_gpu: resources.nodes.get_unwrap(node).into(),
        resources,
      };
      material.setup_pass(pass, &ctx);

      mesh.setup_pass_and_draw(pass, self.group);
    });
  }
}

pub struct OverridableMeshModelImpl {
  inner: MeshModelImpl,
  override_gpu: Option<TransformGPU>,
  overrides: Vec<Box<dyn WorldMatrixOverride>>,
}

impl OverridableMeshModelImpl {
  pub fn push_override(&mut self, o: impl WorldMatrixOverride + 'static) {
    self.overrides.push(Box::new(o));
  }
}

pub trait WorldMatrixOverride {
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32>;
}

impl std::ops::Deref for OverridableMeshModelImpl {
  type Target = MeshModelImpl;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for OverridableMeshModelImpl {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl SceneRenderable for OverridableMeshModelImpl {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let inner = &mut self.inner;
    let material = &mut inner.material;
    let mesh = &mut inner.mesh;

    let mut world_matrix = inner.node.visit(|n| n.world_matrix);

    for override_impl in &self.overrides {
      world_matrix = override_impl.override_mat(world_matrix, base);
    }

    self
      .override_gpu
      .get_or_insert_with(|| TransformGPU::new(gpu, &world_matrix))
      .update(gpu, &world_matrix);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      active_mesh: mesh.as_ref().into(),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    let ctx = SceneMaterialPassSetupCtx {
      camera_gpu,
      model_gpu: self.override_gpu.as_ref(),
      resources,
    };
    material.setup_pass(pass, &ctx);

    mesh.setup_pass_and_draw(pass, self.group);
  }
}

pub struct ViewAutoScalable {
  /// the position by default will choose by the node's world matrix;
  ///
  /// but in sometimes, we need use another position for position
  /// to keep consistent dynamic scale behavior among the group of scene node hierarchy.
  /// in this case, we can use this override_position and update this position manually.
  ///
  pub override_position: Option<Vec3<f32>>,

  pub independent_scale_factor: f32,
}

impl WorldMatrixOverride for Rc<RefCell<ViewAutoScalable>> {
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32> {
    let inner = self.borrow();
    inner.override_mat(world_matrix, base)
  }
}

impl WorldMatrixOverride for ViewAutoScalable {
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32> {
    let camera = &base.camera;

    let center = self
      .override_position
      .unwrap_or_else(|| world_matrix.position());
    let camera_position = camera.node.visit(|n| n.world_matrix.position());
    let distance = (camera_position - center).length();

    let camera_view_height = camera.view_size_in_pixel(base.pass_info.buffer_size).y;

    let scale = self.independent_scale_factor
      / camera
        .projection
        .pixels_per_unit(distance, camera_view_height);

    let raw_scale = world_matrix.extract_scale();
    let new_scale = Vec3::splat(scale) / raw_scale;

    Mat4::scale(new_scale.x, new_scale.y, new_scale.z) * world_matrix
  }
}

pub struct BillBoard {
  /// define what the front direction is (in object space)
  ///
  /// the front_direction will always lookat the view direction
  pub front_direction: Vec3<f32>,
}

impl Default for BillBoard {
  fn default() -> Self {
    Self {
      front_direction: Vec3::new(0., 0., 1.),
    }
  }
}

impl WorldMatrixOverride for BillBoard {
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32> {
    let camera = &base.camera;
    let camera_position = camera.node.visit(|n| n.world_matrix.position());

    let scale = world_matrix.extract_scale();
    let scale = Mat4::scale(scale.x, scale.y, scale.z);
    let position = world_matrix.position();
    let position_m = Mat4::translate(position.x, position.y, position.z);

    let correction = Mat4::lookat(
      Vec3::new(0., 0., 0.),
      self.front_direction,
      Vec3::new(0., 1., 0.),
    );

    let rotation = Mat4::lookat(position, camera_position, Vec3::new(0., 1., 0.));

    // there must be cheaper ways
    position_m * rotation * correction * scale
  }
}
