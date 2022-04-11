use std::{cell::RefCell, rc::Rc};

use rendiation_algebra::*;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{
  IntersectAbleGroupedMesh, MeshBufferHitPoint, MeshBufferIntersectConfig,
};
use rendiation_texture::Size;

use crate::*;

pub type SceneFatlineMaterial = MaterialInner<StateControl<FatLineMaterial>>;

pub type FatlineImpl = MeshModelImpl<MeshInner<FatlineMesh>, SceneFatlineMaterial>;

impl<Me, Ma> SceneRenderable for MeshModel<Me, Ma>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    let inner = self.inner.borrow();
    inner.render(pass, dispatcher, camera)
  }

  fn ray_pick_nearest(
    &self,
    world_ray: &Ray3,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    self.inner.borrow().ray_pick_nearest(world_ray, conf)
  }
}

impl<Me, Ma> SceneRenderableRc for MeshModel<Me, Ma>
where
  Self: SceneRenderable + Clone,
{
  fn id(&self) -> usize {
    self.inner.borrow().id()
  }
  fn clone_boxed(&self) -> Box<dyn SceneRenderableRc> {
    Box::new(self.clone())
  }
  fn as_renderable(&self) -> &dyn SceneRenderable {
    self
  }
  fn as_renderable_mut(&mut self) -> &mut dyn SceneRenderable {
    self
  }
}

impl<Me, Ma> MeshModelImpl<Me, Ma> {
  pub fn into_matrix_overridable(self) -> OverridableMeshModelImpl<Me, Ma> {
    OverridableMeshModelImpl {
      inner: self,
      override_gpu: Default::default(),
      overrides: Vec::with_capacity(1),
    }
  }
}

impl<Me, Ma> MeshModelImpl<Me, Ma>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  fn setup_pass_core(
    &self,
    pass: &mut SceneRenderPass,
    camera: &SceneCamera,
    override_node: Option<&TransformGPU>,
    dispatcher: &dyn RenderComponentAny,
  ) {
    let gpu = pass.ctx.gpu;
    let resources = &mut pass.resources;
    let pass_gpu = dispatcher;
    let camera_gpu = resources.cameras.check_update_gpu(camera, gpu);

    let node_gpu =
      override_node.unwrap_or_else(|| resources.nodes.check_update_gpu(&self.node, gpu));

    let material_gpu =
      self
        .material
        .check_update_gpu(&mut resources.scene.materials, &mut resources.content, gpu);

    let mesh_gpu = self.mesh.check_update_gpu(
      &mut resources.scene.meshes,
      &mut resources.custom_storage,
      gpu,
    );

    let components = [pass_gpu, mesh_gpu, camera_gpu, node_gpu, material_gpu];

    let emitter = MeshDrawcallEmitterWrap {
      group: self.group,
      mesh: &self.mesh,
    };

    RenderEmitter::new(components.as_slice(), &emitter).render(&mut pass.ctx);
  }
}

impl<Me, Ma> SceneRenderable for MeshModelImpl<Me, Ma>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.setup_pass_core(pass, camera, None, dispatcher);
  }

  fn ray_pick_nearest(
    &self,
    world_ray: &Ray3,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    let world_inv = self.node.visit(|n| n.world_matrix).inverse_or_identity();

    let local_ray = world_ray.clone().apply_matrix_into(world_inv);

    if !self.material.is_keep_mesh_shape() {
      return None;
    }

    let mesh = &self.mesh;
    let mut picked = None;
    mesh.try_pick(&mut |mesh: &dyn IntersectAbleGroupedMesh| {
      picked = mesh.intersect_nearest(local_ray, conf, self.group).into();
    });
    picked
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

    let mut world_matrix = self.inner.node.visit(|n| n.world_matrix);
    self
      .overrides
      .iter()
      .for_each(|o| world_matrix = o.override_mat(world_matrix, &ctx));

    let mut override_gpu = self.override_gpu.borrow_mut();
    let node_gpu = override_gpu
      .get_or_insert_with(|| TransformGPU::new(gpu, &world_matrix))
      .update(gpu, &world_matrix);

    self.setup_pass_core(pass, camera, Some(node_gpu), dispatcher);
  }
}

pub struct InverseWorld;

impl WorldMatrixOverride for InverseWorld {
  fn override_mat(&self, world_matrix: Mat4<f32>, _ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    world_matrix.inverse_or_identity()
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
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let inner = self.borrow();
    inner.override_mat(world_matrix, ctx)
  }
}

impl WorldMatrixOverride for ViewAutoScalable {
  fn override_mat(&self, world_matrix: Mat4<f32>, ctx: &WorldMatrixOverrideCtx) -> Mat4<f32> {
    let camera = &ctx.camera;

    let center = self
      .override_position
      .unwrap_or_else(|| world_matrix.position());
    let camera_position = camera.node.visit(|n| n.world_matrix.position());
    let distance = (camera_position - center).length();

    let camera_view_height = camera.view_size_in_pixel(ctx.buffer_size).y;

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
