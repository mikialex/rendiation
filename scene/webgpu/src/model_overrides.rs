use crate::*;

pub trait WebGPUModelExt<Me, Ma> {
  fn into_matrix_overridable(self) -> OverridableMeshModelImpl<Me, Ma>;
}

impl<Me, Ma> WebGPUModelExt<Me, Ma> for MeshModelImpl<Me, Ma> {
  fn into_matrix_overridable(self) -> OverridableMeshModelImpl<Me, Ma> {
    OverridableMeshModelImpl {
      inner: self,
      override_gpu: Default::default(),
      overrides: Vec::with_capacity(1),
    }
  }
}

pub struct OverridableMeshModelImpl<Me, Ma> {
  inner: MeshModelImpl<Me, Ma>,
  override_gpu: RefCell<Option<TransformGPU>>,
  overrides: Vec<Box<dyn WorldMatrixOverride>>,
}

impl<Me, Ma> OverridableMeshModelImpl<Me, Ma> {
  pub fn push_override(&mut self, o: impl WorldMatrixOverride + 'static) {
    self.overrides.push(Box::new(o));
  }

  pub fn compute_override_world_mat(&self, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let mut world_matrix = self.inner.node.get_world_matrix();
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
  pub camera: &'a Camera,
  pub buffer_size: Size,
}

impl<Me, Ma> std::ops::Deref for OverridableMeshModelImpl<Me, Ma> {
  type Target = MeshModelImpl<Me, Ma>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<Me, Ma> std::ops::DerefMut for OverridableMeshModelImpl<Me, Ma> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<Me, Ma> SceneNodeControlled for OverridableMeshModelImpl<Me, Ma> {
  fn visit_node(&self, visitor: &mut dyn FnMut(&SceneNode)) {
    visitor(&self.inner.node)
  }
}

impl<Me: WebGPUSceneMesh, Ma: WebGPUSceneMaterial> SceneRenderable
  for OverridableMeshModelImpl<Me, Ma>
{
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    let gpu = pass.ctx.gpu;

    let ctx = WorldMatrixOverrideCtx {
      camera,
      buffer_size: pass.size(),
    };

    let world_matrix = self.compute_override_world_mat(&ctx);

    let mut override_gpu = self.override_gpu.borrow_mut();
    let node_gpu = override_gpu
      .get_or_insert_with(|| TransformGPU::new(gpu, &world_matrix))
      .update(gpu, &world_matrix);

    setup_pass_core(self, pass, camera, Some(node_gpu), dispatcher);
  }
}

impl<Me: WebGPUSceneMesh, Ma: WebGPUSceneMaterial> SceneRayInteractive
  for OverridableMeshModelImpl<Me, Ma>
{
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    let o_ctx = WorldMatrixOverrideCtx {
      camera: ctx.camera,
      buffer_size: ctx.camera_view_size,
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
  pub fn get_optional_position(&self) -> Option<Vec3<f32>> {
    match self {
      ViewAutoScalablePositionOverride::None => None,
      ViewAutoScalablePositionOverride::Fixed(f) => Some(*f),
      ViewAutoScalablePositionOverride::SyncNode(n) => Some(n.get_world_matrix().position()),
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
      .get_optional_position()
      .unwrap_or_else(|| world_matrix.position());
    let world_translation = Mat4::translate(world_position.x, world_position.y, world_position.z);

    let camera_position = camera.node.get_world_matrix().position();
    let distance = (camera_position - world_position).length();

    let camera_view_height = camera.view_size_in_pixel(ctx.buffer_size).y;

    let scale = self.independent_scale_factor
      / camera
        .projection
        .pixels_per_unit(distance, camera_view_height);

    let raw_scale = world_matrix.extract_scale();
    let new_scale = Vec3::splat(scale) / raw_scale;

    world_translation // move back to position
      * Mat4::scale(new_scale.x, new_scale.y, new_scale.z) // apply new scale
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
