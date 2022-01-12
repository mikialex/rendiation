use std::{any::Any, cell::RefCell, ops::Deref, rc::Rc};

use rendiation_algebra::*;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::mesh::{
  IntersectAbleGroupedMesh, MeshBufferHitPoint, MeshBufferIntersectConfig,
};
use rendiation_webgpu::{GPURenderPass, GPU};

use crate::*;

pub type SceneFatlineMaterial = MaterialInner<SceneMaterial<FatLineMaterial>>;

pub type FatlineImpl = MeshModelImpl<MeshInner<FatlineMesh>, SceneFatlineMaterial>;

impl<Me, Ma> SceneRenderable for MeshModel<Me, Ma>
where
  Me: WebGPUMesh,
  Ma: WebGPUMaterial,
{
  fn update(
    &self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    res: &mut GPUResourceSceneCache,
  ) {
    let inner = self.inner.borrow();
    inner.update(gpu, base, res)
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

pub trait WebGPUMaterial: Any {
  fn update(
    &self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    res: &mut GPUResourceSceneCache,
  );

  fn setup_pass<'a>(
    &self,
    res: &GPUResourceSceneCache,
    pass: &mut GPURenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx,
  );

  fn is_keep_mesh_shape(&self) -> bool;
}

impl<T: MaterialCPUResource> WebGPUMaterial for MaterialInner<T> {
  fn update(
    &self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    res: &mut GPUResourceSceneCache,
  ) {
    res.update_material(self, gpu, ctx)
  }

  fn setup_pass<'a>(
    &self,
    res: &GPUResourceSceneCache,
    pass: &mut GPURenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx,
  ) {
    res.setup_material(self, pass, ctx)
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.deref().is_keep_mesh_shape()
  }
}

impl<Me, Ma> SceneRenderable for MeshModelImpl<Me, Ma>
where
  Me: WebGPUMesh,
  Ma: WebGPUMaterial,
{
  fn update(
    &self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    res: &mut GPUResourceSceneCache,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;
    let mesh_dyn: &dyn WebGPUMesh = mesh;

    self.node.check_update_gpu(base.resources, gpu);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      active_mesh: mesh_dyn.into(),
    };

    material.update(gpu, &mut ctx, res);
    mesh.update(gpu, &mut base.resources.custom_storage, res);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let material = &self.material;

    self.node.visit(|node| {
      let model_gpu = resources.content.nodes.get_unwrap(node).into();
      let ctx = SceneMaterialPassSetupCtx {
        camera_gpu,
        model_gpu,
        resources: &resources.content,
      };
      material.setup_pass(&resources.scene, pass, &ctx);
      self
        .mesh
        .setup_pass_and_draw(pass, self.group, &resources.scene);
    });
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
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32>;
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

impl<Me: WebGPUMesh, Ma: WebGPUMaterial> SceneRenderable for OverridableMeshModelImpl<Me, Ma> {
  fn update(
    &self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    res: &mut GPUResourceSceneCache,
  ) {
    let inner = &self.inner;
    let material = &inner.material;
    let mesh = &inner.mesh;
    let mesh_dyn: &dyn WebGPUMesh = mesh;

    let mut world_matrix = inner.node.visit(|n| n.world_matrix);

    for override_impl in &self.overrides {
      world_matrix = override_impl.override_mat(world_matrix, base);
    }

    self
      .override_gpu
      .borrow_mut()
      .get_or_insert_with(|| TransformGPU::new(gpu, &world_matrix))
      .update(gpu, &world_matrix);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      active_mesh: mesh_dyn.into(),
    };

    material.update(gpu, &mut ctx, res);

    mesh.update(gpu, &mut base.resources.custom_storage, res);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut SceneRenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
  ) {
    let material = &self.material;

    let override_gpu = self.override_gpu.borrow();
    let ctx = SceneMaterialPassSetupCtx {
      camera_gpu,
      model_gpu: override_gpu.as_ref(),
      resources: &resources.content,
    };
    material.setup_pass(&resources.scene, pass, &ctx);
    self
      .mesh
      .setup_pass_and_draw(pass, self.group, &resources.scene);
  }
}

pub struct InverseWorld;

impl WorldMatrixOverride for InverseWorld {
  fn override_mat(
    &self,
    world_matrix: Mat4<f32>,
    _base: &mut SceneMaterialRenderPrepareCtxBase,
  ) -> Mat4<f32> {
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
