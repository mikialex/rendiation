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
    let inner = self.inner.read().unwrap();
    inner.render(pass, dispatcher, camera)
  }

  fn ray_pick_nearest(
    &self,
    world_ray: &Ray3,
    conf: &MeshBufferIntersectConfig,
  ) -> Option<Nearest<MeshBufferHitPoint>> {
    self.inner.read().unwrap().ray_pick_nearest(world_ray, conf)
  }
}

impl<Me, Ma> SceneRenderableShareable for MeshModel<Me, Ma>
where
  Self: SceneRenderable + Clone,
{
  fn id(&self) -> usize {
    self.inner.read().unwrap().id()
  }
  fn clone_boxed(&self) -> Box<dyn SceneRenderableShareable> {
    Box::new(self.clone())
  }
  fn as_renderable(&self) -> &dyn SceneRenderable {
    self
  }
  fn as_renderable_mut(&mut self) -> &mut dyn SceneRenderable {
    self
  }
}

pub fn setup_pass_core<Me, Ma>(
  model: &MeshModelImpl<Me, Ma>,
  pass: &mut SceneRenderPass,
  camera: &SceneCamera,
  override_node: Option<&TransformGPU>,
  dispatcher: &dyn RenderComponentAny,
) where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  let gpu = pass.ctx.gpu;
  let resources = &mut pass.resources;
  let pass_gpu = dispatcher;
  let camera_gpu = resources.cameras.check_update_gpu(camera, gpu);

  let net_visible = model.node.visit(|n| n.net_visible);
  if !net_visible {
    return;
  }

  let node_gpu =
    override_node.unwrap_or_else(|| resources.nodes.check_update_gpu(&model.node, gpu));

  let material_gpu =
    model
      .material
      .check_update_gpu(&mut resources.scene.materials, &mut resources.content, gpu);

  let mesh_gpu = model.mesh.check_update_gpu(
    &mut resources.scene.meshes,
    &mut resources.custom_storage,
    gpu,
  );

  let components = [pass_gpu, mesh_gpu, node_gpu, camera_gpu, material_gpu];

  let emitter = MeshDrawcallEmitterWrap {
    group: model.group,
    mesh: &model.mesh,
  };

  RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &emitter);
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
    setup_pass_core(self, pass, camera, None, dispatcher);
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

// pub struct InteractiveWatchable<T> {
//   inner: T,
//   callbacks: Vec<Box<dyn Fn(&T)>>,
// }

// impl<T> InteractiveWatchable<T> {
//   pub fn on(&mut self, cb: impl Fn(&T) + 'static) -> &mut Self {
//     self.callbacks.push(Box::new(cb));
//     self
//   }
// }

// pub trait InteractiveWatchableInit<T> {
//   fn interactive_watchable(self) -> InteractiveWatchable<T>;
// }

// impl<T: SceneRenderable> InteractiveWatchableInit<T> for T {
//   fn interactive_watchable(self) -> InteractiveWatchable<T> {
//     InteractiveWatchable {
//       inner: self,
//       callbacks: Default::default(),
//     }
//   }
// }

// impl<T: SceneRenderable> SceneRenderable for InteractiveWatchable<T> {
//   fn render(
//     &self,
//     pass: &mut SceneRenderPass,
//     dispatcher: &dyn RenderComponentAny,
//     camera: &SceneCamera,
//   ) {
//     self.inner.render(pass, dispatcher, camera)
//   }

//   fn ray_pick_nearest(
//     &self,
//     _world_ray: &Ray3,
//     _conf: &MeshBufferIntersectConfig,
//   ) -> Option<Nearest<MeshBufferHitPoint>> {
//     None
//   }

//   fn get_bounding_info(&self) -> Option<Box3> {
//     self.inner.get_bounding_info()
//   }
// }
