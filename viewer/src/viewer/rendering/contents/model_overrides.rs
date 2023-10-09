use std::rc::Rc;

use __core::cell::RefCell;
use rendiation_geometry::OptionalNearest;
use rendiation_mesh_core::MeshBufferHitPoint;
use rendiation_scene_interaction::*;
use rendiation_shader_api::*;
use webgpu::*;

use crate::*;

pub trait WebGPUModelExt {
  fn into_matrix_overridable(self) -> OverridableMeshModelImpl;
}

impl WebGPUModelExt for SceneModelImpl {
  fn into_matrix_overridable(self) -> OverridableMeshModelImpl {
    OverridableMeshModelImpl {
      inner: self,
      override_gpu: Default::default(),
      overrides: Vec::with_capacity(1),
    }
  }
}

pub struct OverridableMeshModelImpl {
  inner: SceneModelImpl,
  override_gpu: RefCell<Option<NodeGPU>>,
  overrides: Vec<Box<dyn WorldMatrixOverride>>,
}

impl OverridableMeshModelImpl {
  pub fn push_override(&mut self, o: impl WorldMatrixOverride + 'static) {
    self.overrides.push(Box::new(o));
  }

  pub fn compute_override_world_mat(&self, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let mut world_matrix = ctx.node_derives.get_world_matrix(&self.inner.node);
    self
      .overrides
      .iter()
      .for_each(|o| world_matrix = o.override_mat(world_matrix, ctx));
    world_matrix
  }
}

pub trait WorldMatrixOverride {
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32>;
}

pub struct WorldMatrixOverrideCtx<'a> {
  pub camera: &'a SceneCameraImpl,
  pub node_derives: &'a SceneNodeDeriveSystem,
  pub buffer_size: Size,
}

impl std::ops::Deref for OverridableMeshModelImpl {
  type Target = SceneModelImpl;

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
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    let gpu = pass.ctx.gpu;

    let camera_ref = camera.read();
    let ctx = WorldMatrixOverrideCtx {
      camera: &camera_ref,
      buffer_size: pass.size(),
      node_derives: scene.node_derives,
    };

    let world_matrix = self.compute_override_world_mat(&ctx);

    let mut override_gpu = self.override_gpu.borrow_mut();
    let node_gpu = override_gpu
      .get_or_insert_with(|| NodeGPU::new(&gpu.device))
      .update(&gpu.queue, world_matrix);

    setup_pass_core(self, pass, camera, Some(node_gpu), dispatcher, scene);
  }
}

impl SceneRayInteractive for OverridableMeshModelImpl {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    let camera_ref = ctx.camera.read();
    let o_ctx = WorldMatrixOverrideCtx {
      camera: &camera_ref,
      buffer_size: ctx.camera_view_size,
      node_derives: ctx.node_derives,
    };

    let world_matrix = self.compute_override_world_mat(&o_ctx);
    ray_pick_nearest_core(self, ctx, world_matrix)
  }
}

pub struct InverseWorld;

impl WorldMatrixOverride for InverseWorld {
  fn override_mat(&self, world_matrix: Mat4<f32>, _ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    world_matrix.inverse_or_identity()
  }
}

/// the position by default will choose by the node's world matrix;
///
/// but in sometimes, we need use another position for position
/// to keep consistent dynamic scale behavior among the group of scene node hierarchy.
/// in this case, we can use this override_position and update this position manually.
pub enum ViewAutoScalablePositionOverride {
  None,
  Fixed(Vec3<f32>),
  SyncNode(SceneNode),
}

impl ViewAutoScalablePositionOverride {
  pub fn get_optional_position(&self, derives: &SceneNodeDeriveSystem) -> Option<Vec3<f32>> {
    match self {
      ViewAutoScalablePositionOverride::None => None,
      ViewAutoScalablePositionOverride::Fixed(f) => Some(*f),
      ViewAutoScalablePositionOverride::SyncNode(n) => Some(derives.get_world_matrix(n).position()),
    }
  }
}

pub struct ViewAutoScalable {
  pub override_position: ViewAutoScalablePositionOverride,

  pub independent_scale_factor: f32,
}

impl WorldMatrixOverride for Rc<RefCell<ViewAutoScalable>> {
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let inner = self.borrow();
    inner.override_mat(world_matrix, ctx)
  }
}

impl WorldMatrixOverride for ViewAutoScalable {
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let camera = &ctx.camera;

    let world_position = self
      .override_position
      .get_optional_position(ctx.node_derives)
      .unwrap_or_else(|| world_matrix.position());
    let world_translation = Mat4::translate(world_position);

    let camera_world = ctx.node_derives.get_world_matrix(&camera.node);
    let camera_position = camera_world.position();
    let camera_forward = camera_world.forward().reverse().normalize();
    let camera_to_target = world_position - camera_position;

    let projected_distance = camera_to_target.dot(camera_forward);

    let camera_view_height = camera.view_size_in_pixel(ctx.buffer_size).y;

    let scale = self.independent_scale_factor
      / camera
        .projection
        .pixels_per_unit(projected_distance, camera_view_height)
        .unwrap_or(1.);

    world_translation // move back to position
      * Mat4::scale(Vec3::splat(scale)) // apply new scale
      * world_translation.inverse_or_identity() // move back to zero
      * world_matrix // original
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
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let camera = &ctx.camera;
    let camera_position = ctx.node_derives.get_world_matrix(&camera.node).position();

    let scale = world_matrix.get_scale();
    let scale = Mat4::scale(scale);
    let position = world_matrix.position();
    let position_m = Mat4::translate(position);

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
