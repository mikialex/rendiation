use std::cell::RefCell;

use rendiation_algebra::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_webgpu::GPURenderPass;

use super::*;

#[derive(Clone)]
pub struct MeshModel {
  pub inner: Rc<RefCell<MeshModelInner>>,
}

impl MeshModel {
  // todo add type constraint
  pub fn new<Ma: Material + 'static, Me: Mesh + 'static>(
    material: Ma,
    mesh: Me,
    node: SceneNode,
  ) -> Self {
    let inner = MeshModelInner::new(material, mesh, node);
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

impl SceneRenderable for MeshModel {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, base)
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let inner = self.inner.borrow();
    inner.setup_pass(pass, camera_gpu, resources, pass_info)
  }
}

pub struct MeshModelInner {
  pub material: Box<dyn Material>,
  pub mesh: Box<dyn Mesh>,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl MeshModelInner {
  // todo add type constraint
  pub fn new<Ma: Material + 'static, Me: Mesh + 'static>(
    material: Ma,
    mesh: Me,
    node: SceneNode,
  ) -> Self {
    Self {
      material: Box::new(material),
      mesh: Box::new(mesh),
      group: Default::default(),
      node,
    }
  }

  pub fn into_auto_scale(self) -> AutoScalableMeshModelInner {
    AutoScalableMeshModelInner {
      inner: self,
      override_gpu: None,
      override_position: None,
      independent_scale_factor: 1.,
    }
  }
}

impl SceneRenderable for MeshModelInner {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let material = &mut self.material;
    let mesh = &mut self.mesh;

    self.node.mutate(|node| {
      let mut ctx = SceneMaterialRenderPrepareCtx {
        base,
        model_info: node.get_model_gpu(gpu).into(),
        active_mesh: mesh.as_ref().into(),
      };

      material.update(gpu, &mut ctx);

      mesh.update(gpu, &mut base.resources.custom_storage);
    });
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    self.node.visit(|node| {
      let ctx = SceneMaterialPassSetupCtx {
        pass: pass_info,
        camera_gpu,
        model_gpu: node.gpu.as_ref().unwrap().into(),
        resources,
        active_mesh: mesh.as_ref().into(),
      };
      material.setup_pass(pass, &ctx);

      mesh.setup_pass_and_draw(pass, self.group);
    });
  }
}

pub struct AutoScalableMeshModelInner {
  inner: MeshModelInner,
  override_gpu: Option<TransformGPU>,

  /// the position by default will choose by the node's world matrix;
  ///
  /// but in sometimes, we need use another position for position
  /// to keep consistent dynamic scale behavior among the group of scene node hierarchy.
  /// in this case, we can use this override_position and update this position manually.
  ///
  pub override_position: Option<Vec3<f32>>,

  pub independent_scale_factor: f32,
}

impl std::ops::Deref for AutoScalableMeshModelInner {
  type Target = MeshModelInner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for AutoScalableMeshModelInner {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl SceneRenderable for AutoScalableMeshModelInner {
  fn update(&mut self, gpu: &GPU, base: &mut SceneMaterialRenderPrepareCtxBase) {
    let inner = &mut self.inner;
    let material = &mut inner.material;
    let mesh = &mut inner.mesh;

    let mut world_matrix = inner.node.visit(|n| n.world_matrix);

    let center = self
      .override_position
      .unwrap_or_else(|| world_matrix.position());
    let camera = base.active_camera.node.visit(|n| n.world_matrix.position());
    let distance = (camera - center).length();

    let scale = self.independent_scale_factor * 1.
      / base
        .active_camera
        .projection
        .pixels_per_unit(distance, 1000.); // todo

    let raw_scale = world_matrix.extract_scale();
    let new_scale = Vec3::splat(scale) / raw_scale;

    world_matrix = Mat4::scale(new_scale.x, new_scale.y, new_scale.z) * world_matrix;

    let transform = self
      .override_gpu
      .get_or_insert_with(|| TransformGPU::new(gpu, &world_matrix))
      .update(gpu, &world_matrix);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      model_info: Some(transform),
      active_mesh: mesh.as_ref().into(),
    };

    material.update(gpu, &mut ctx);

    mesh.update(gpu, &mut base.resources.custom_storage);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let material = &self.material;
    let mesh = &self.mesh;

    let ctx = SceneMaterialPassSetupCtx {
      pass: pass_info,
      camera_gpu,
      model_gpu: self.override_gpu.as_ref(),
      resources,
      active_mesh: mesh.as_ref().into(),
    };
    material.setup_pass(pass, &ctx);

    mesh.setup_pass_and_draw(pass, self.group);
  }
}
