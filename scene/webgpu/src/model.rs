use crate::*;

pub type SceneFatlineMaterial = StateControl<FatLineMaterial>;

pub type FatlineImpl = MeshModelImpl<FatlineMesh, SceneFatlineMaterial>;

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
    self.visit(|model| model.render(pass, dispatcher, camera))
  }
}

impl<Me, Ma> SceneRayInteractive for MeshModel<Me, Ma>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.visit(|model| model.ray_pick_nearest(ctx))
  }
}

impl<Me, Ma> SceneNodeControlled for MeshModel<Me, Ma> {
  fn visit_node(&self, visitor: &mut dyn FnMut(&SceneNode)) {
    self.visit(|model| visitor(&model.node))
  }
}

impl<Me, Ma> SceneRenderableShareable for MeshModel<Me, Ma>
where
  Self: SceneRenderable + Clone + 'static,
{
  fn id(&self) -> usize {
    self.read().unwrap().id()
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

  let material = model.material.read().unwrap();
  let material_gpu =
    material.check_update_gpu(&mut resources.scene.materials, &mut resources.content, gpu);

  let mesh = model.mesh.read().unwrap();
  let mesh_gpu = mesh.check_update_gpu(
    &mut resources.scene.meshes,
    &mut resources.custom_storage,
    gpu,
  );

  let components = [pass_gpu, mesh_gpu, node_gpu, camera_gpu, material_gpu];

  let mesh: &dyn MeshDrawcallEmitter = mesh.deref().deref();
  let emitter = MeshDrawcallEmitterWrap {
    group: model.group,
    mesh,
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
}

pub fn ray_pick_nearest_core<Me, Ma>(
  model: &MeshModelImpl<Me, Ma>,
  ctx: &SceneRayInteractiveCtx,
  world_mat: Mat4<f32>,
) -> OptionalNearest<MeshBufferHitPoint>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  let net_visible = model.node.visit(|n| n.net_visible);
  if !net_visible {
    return OptionalNearest::none();
  }

  let world_inv = world_mat.inverse_or_identity();

  let local_ray = ctx.world_ray.clone().apply_matrix_into(world_inv);

  if !model.material.read().unwrap().is_keep_mesh_shape() {
    return OptionalNearest::none();
  }

  let mesh = &model.mesh.read().unwrap();
  let mut picked = OptionalNearest::none();
  mesh.try_pick(&mut |mesh: &dyn IntersectAbleGroupedMesh| {
    picked = mesh.intersect_nearest(local_ray, ctx.conf, model.group);

    // transform back to world space
    if let Some(result) = &mut picked.0 {
      let hit = &mut result.hit;
      hit.position = world_mat * hit.position;
      hit.distance = (hit.position - ctx.world_ray.origin).length()
    }
  });
  picked
}

impl<Me, Ma> SceneRayInteractive for MeshModelImpl<Me, Ma>
where
  Me: WebGPUSceneMesh,
  Ma: WebGPUSceneMaterial,
{
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    ray_pick_nearest_core(self, ctx, self.node.get_world_matrix())
  }
}
