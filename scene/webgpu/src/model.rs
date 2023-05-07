use crate::*;

impl SceneRenderable for SceneModel {
  fn is_transparent(&self) -> bool {
    self.visit(|model| model.is_transparent())
  }
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.visit(|model| model.render(pass, dispatcher, camera))
  }
}

impl SceneRayInteractive for SceneModel {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.visit(|model| model.ray_pick_nearest(ctx))
  }
}

impl SceneNodeControlled for SceneModel {
  fn visit_node(&self, visitor: &mut dyn FnMut(&SceneNode)) {
    self.visit(|model| visitor(&model.node))
  }
}

impl SceneRenderableShareable for SceneModel
where
  Self: SceneRenderable + Clone + 'static,
{
  fn id(&self) -> usize {
    self.read().guid()
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

pub fn setup_pass_core(
  model_input: &SceneModelImpl,
  pass: &mut SceneRenderPass,
  camera: &SceneCamera,
  override_node: Option<&NodeGPU>,
  dispatcher: &dyn RenderComponentAny,
) {
  match &model_input.model {
    ModelType::Standard(model) => {
      let model = model.read();
      let pass_gpu = dispatcher;

      let camera_gpu = pass.scene_resources.cameras.get_camera_gpu(camera).unwrap();

      let net_visible = pass.node_derives.get_net_visible(&model_input.node);
      if !net_visible {
        return;
      }

      let node_gpu = override_node.unwrap_or(
        pass
          .scene_resources
          .nodes
          .get_node_gpu(&model_input.node)
          .unwrap(),
      );

      let mut materials = pass.resources.model_ctx.materials.write().unwrap();
      let material_gpu = materials.get_or_insert_with(model.material.guid().unwrap(), || {
        model
          .material
          .create_scene_reactive_gpu(&pass.resources.bindable_ctx)
          .unwrap()
      });

      let mut meshes = pass.resources.model_ctx.meshes.write().unwrap();
      let mesh_gpu = meshes.get_or_insert_with(model.mesh.guid().unwrap(), || {
        model
          .mesh
          .create_scene_reactive_gpu(&pass.resources.bindable_ctx)
          .unwrap()
      });

      let components = [pass_gpu, mesh_gpu, node_gpu, camera_gpu, material_gpu];

      let mesh: &dyn MeshDrawcallEmitter = &model.mesh;
      let emitter = MeshDrawcallEmitterWrap {
        group: model.group,
        mesh,
      };

      RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &emitter);
    }
    ModelType::Foreign(model) => {
      if let Some(model) = model.downcast_ref::<Box<dyn SceneRenderable>>() {
        model.render(pass, dispatcher, camera)
      }
    }
    _ => {}
  };
}

impl SceneRenderable for SceneModelImpl {
  fn is_transparent(&self) -> bool {
    match &self.model {
      ModelType::Standard(model) => model.read().material.is_transparent(),
      ModelType::Foreign(model) => {
        if let Some(model) = model.downcast_ref::<Box<dyn SceneRenderable>>() {
          model.is_transparent()
        } else {
          false
        }
      }
      _ => false,
    }
  }
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    setup_pass_core(self, pass, camera, None, dispatcher);
  }
}

pub fn ray_pick_nearest_core(
  m: &SceneModelImpl,
  ctx: &SceneRayInteractiveCtx,
  world_mat: Mat4<f32>,
) -> OptionalNearest<MeshBufferHitPoint> {
  match &m.model {
    ModelType::Standard(model) => {
      let net_visible = ctx.node_derives.get_net_visible(&m.node);
      if !net_visible {
        return OptionalNearest::none();
      }

      let world_inv = world_mat.inverse_or_identity();

      let local_ray = ctx.world_ray.clone().apply_matrix_into(world_inv);

      let model = model.read();

      if !model.material.is_keep_mesh_shape() {
        return OptionalNearest::none();
      }

      let mut picked = OptionalNearest::none();
      model
        .mesh
        .try_pick(&mut |mesh: &dyn IntersectAbleGroupedMesh| {
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
    ModelType::Foreign(model) => {
      // todo should merge vtable to render
      if let Some(model) = model.downcast_ref::<Box<dyn SceneRayInteractive>>() {
        model.ray_pick_nearest(ctx)
      } else {
        OptionalNearest::none()
      }
    }
    _ => OptionalNearest::none(),
  }
}

impl SceneRayInteractive for SceneModelImpl {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    ray_pick_nearest_core(self, ctx, ctx.node_derives.get_world_matrix(&self.node))
  }
}

#[pin_project::pin_project]
pub struct StandardModelGPU {
  material_id: Option<usize>,
  material_delta: Option<ReactiveMaterialRenderComponentDeltaSource>,
  mesh_id: Option<usize>,
  mesh_delta: Option<ReactiveMeshRenderComponentDeltaSource>,
}

impl Stream for StandardModelGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.material_delta, cx);
    early_return_option_ready!(this.mesh_delta, cx);
    Poll::Pending
  }
}

pub type ReactiveStandardModelGPU = impl AsRef<RenderComponentCell<StandardModelGPU>>
  + Stream<Item = RenderComponentDeltaFlag>
  + Unpin;

pub fn build_standard_model_gpu(
  source: &SceneItemRef<StandardModel>,
  ctx: &GPUModelResourceCtx,
) -> ReactiveStandardModelGPU {
  let s = source.read();
  let gpu = StandardModelGPU {
    material_id: s.material.guid(),
    material_delta: ctx.get_or_create_reactive_material_render_component_delta_source(&s.material),
    mesh_id: s.mesh.guid(),
    mesh_delta: ctx.get_or_create_reactive_mesh_render_component_delta_source(&s.mesh),
  };

  let state = RenderComponentCell::new(gpu);
  let ctx = ctx.clone();

  source
    .unbound_listen_by(all_delta)
    .fold_signal_flatten(state, move |delta, state| match delta {
      StandardModelDelta::material(material) => {
        state.inner.material_id = material.guid();
        state.inner.material_delta =
          ctx.get_or_create_reactive_material_render_component_delta_source(&material);
        RenderComponentDeltaFlag::ContentRef
      }
      StandardModelDelta::mesh(mesh) => {
        state.inner.mesh_id = mesh.guid();
        state.inner.mesh_delta =
          ctx.get_or_create_reactive_mesh_render_component_delta_source(&mesh);
        RenderComponentDeltaFlag::ContentRef
      }
      StandardModelDelta::group(_) => RenderComponentDeltaFlag::Draw,
      StandardModelDelta::skeleton(_) => RenderComponentDeltaFlag::all(),
    })
}

#[pin_project::pin_project(project = ReactiveSceneModelGPUTypeProj)]
pub enum ReactiveSceneModelGPUType {
  Standard(ReactiveStandardModelGPU),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
}

impl ReactiveSceneModelGPUType {
  pub fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    match self {
      ReactiveSceneModelGPUType::Standard(m) => {
        Box::pin(m.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      ReactiveSceneModelGPUType::Foreign(m) => m
        .as_reactive_component()
        .create_render_component_delta_stream(),
    }
  }
}

impl Stream for ReactiveSceneModelGPUType {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    match self.project() {
      ReactiveSceneModelGPUTypeProj::Standard(m) => m.poll_next_unpin(cx),
      ReactiveSceneModelGPUTypeProj::Foreign(m) => m.poll_next_unpin(cx),
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveSceneModelGPU {
  node_id: usize, // todo add stream here
  model_id: Option<usize>,
  model_delta: Option<ReactiveModelRenderComponentDeltaSource>,
}

impl Stream for ReactiveSceneModelGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.model_delta, cx);
    Poll::Pending
  }
}

pub trait WebGPUSceneModel: Send + Sync {
  fn create_scene_reactive_gpu(
    &self,
    ctx: &GPUModelResourceCtx,
  ) -> Option<ReactiveSceneModelGPUType>;
}
define_dyn_trait_downcaster_static!(WebGPUSceneModel);
pub fn register_webgpu_model_features<T>()
where
  T: AsRef<dyn WebGPUSceneModel> + AsMut<dyn WebGPUSceneModel> + 'static,
{
  get_dyn_trait_downcaster_static!(WebGPUSceneModel).register::<T>()
}

impl WebGPUSceneModel for ModelType {
  fn create_scene_reactive_gpu(
    &self,
    ctx: &GPUModelResourceCtx,
  ) -> Option<ReactiveSceneModelGPUType> {
    match self {
      Self::Standard(model) => {
        ReactiveSceneModelGPUType::Standard(build_standard_model_gpu(model, ctx))
      }
      Self::Foreign(m) => {
        return if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneModel>>() {
          m.create_scene_reactive_gpu(ctx)
        } else {
          None
        }
      }
      _ => return None,
    }
    .into()
  }
}

pub type ReactiveSceneModelGPUInstance =
  impl AsRef<RenderComponentCell<ReactiveSceneModelGPU>> + Stream<Item = RenderComponentDeltaFlag>;

pub fn build_scene_model_gpu(
  source: &SceneModel,
  ctx: &ContentGPUSystem,
) -> ReactiveSceneModelGPUInstance {
  let source = source.read();

  let model_id = source.model.guid();
  let model_delta = ctx.get_or_create_reactive_model_render_component_delta_source(&source.model);

  let instance = ReactiveSceneModelGPU {
    node_id: source.node.guid(),
    model_id,
    model_delta,
  };

  let state = RenderComponentCell::new(instance);
  let ctx = ctx.clone();

  source
    .unbound_listen_by(all_delta)
    .fold_signal_flatten(state, move |v, state| match v {
      SceneModelImplDelta::model(model) => {
        let model_id = model.guid();
        let model_delta = ctx.get_or_create_reactive_model_render_component_delta_source(&model);
        state.inner.model_id = model_id;
        state.inner.model_delta = model_delta;
        RenderComponentDeltaFlag::ContentRef
      }
      SceneModelImplDelta::node(_) => {
        // todo, handle node change
        RenderComponentDeltaFlag::ContentRef
      }
    })
}
