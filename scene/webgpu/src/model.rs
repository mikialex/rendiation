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
    self.read().id()
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
      let resources = &mut pass.resources;
      let pass_gpu = dispatcher;
      let camera_gpu = resources
        .cameras
        .get_with_update(camera, &(pass.ctx.gpu, pass.node_derives));

      let net_visible = pass.node_derives.get_net_visible(&model_input.node);
      if !net_visible {
        return;
      }

      let node_gpu = override_node.unwrap_or_else(|| {
        resources
          .nodes
          .get_with_update(&model_input.node, &(pass.ctx.gpu, pass.node_derives))
      });

      let mut materials = resources.materials.write().unwrap();
      let material_gpu = materials.get_or_insert_with(model.material.id().unwrap(), || {
        model
          .material
          .create_scene_reactive_gpu(&resources.bindables)
          .unwrap()
      });

      let mut meshes = resources.meshes.write().unwrap();
      let mesh_gpu = meshes.get_or_insert_with(model.mesh.id().unwrap(), || {
        model
          .mesh
          .create_scene_reactive_gpu(&resources.bindables)
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
    material_id: s.material.id(),
    material_delta: ctx.get_or_create_reactive_material_render_component_delta_source(&s.material),
    mesh_id: s.mesh.id(),
    mesh_delta: ctx.get_or_create_reactive_mesh_render_component_delta_source(&s.mesh),
  };

  let state = RenderComponentCell::new(gpu);
  let ctx = ctx.clone();

  source
    .unbound_listen_by(all_delta)
    .fold_signal_flatten(state, move |delta, state| match delta {
      StandardModelDelta::material(material) => {
        state.inner.material_id = material.id();
        state.inner.material_delta =
          ctx.get_or_create_reactive_material_render_component_delta_source(&material);
        RenderComponentDeltaFlag::ContentRef
      }
      StandardModelDelta::mesh(mesh) => {
        state.inner.mesh_id = mesh.id();
        state.inner.mesh_delta =
          ctx.get_or_create_reactive_mesh_render_component_delta_source(&mesh);
        RenderComponentDeltaFlag::ContentRef
      }
      StandardModelDelta::group(_) => RenderComponentDeltaFlag::Draw,
      StandardModelDelta::skeleton(_) => todo!(),
    })
}

#[pin_project::pin_project(project = ReactiveSceneModelGPUTypeProj)]
pub enum ReactiveSceneModelGPUType {
  Standard(ReactiveStandardModelGPU),
  Foreign,
}

impl ReactiveSceneModelGPUType {
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    todo!()
  }
}

impl Stream for ReactiveSceneModelGPUType {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    match self.project() {
      ReactiveSceneModelGPUTypeProj::Standard(m) => m.poll_next_unpin(cx),
      // ReactiveSceneModelGPUTypeProj::Foreign(m) => m.poll_next_unpin(cx),
      _ => todo!(),
    }
  }
}

#[pin_project::pin_project]
pub struct ReactiveSceneModelGPU {
  node_id: usize, // todo add stream here
  model_id: Option<usize>,
  model_delta: Option<Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>>,
}

impl Stream for ReactiveSceneModelGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.model_delta, cx);
    Poll::Pending
  }
}

fn build_model_gpu(
  model: &ModelType,
  ctx: &GPUModelResourceCtx,
) -> Option<ReactiveSceneModelGPUType> {
  match model {
    ModelType::Standard(model) => {
      ReactiveSceneModelGPUType::Standard(build_standard_model_gpu(model, ctx)).into()
    }
    ModelType::Foreign(_) => None,
    _ => None,
  }
}

pub type ReactiveSceneModelGPUInstance =
  impl AsRef<RenderComponentCell<ReactiveSceneModelGPU>> + Stream<Item = RenderComponentDeltaFlag>;

pub fn build_scene_model_gpu(
  source: &SceneModel,
  ctx: &GPUModelResourceCtx,
  models: &mut StreamMap<ReactiveSceneModelGPUType>,
) -> ReactiveSceneModelGPUInstance {
  let source = source.read();

  let model_id = match &source.model {
    ModelType::Standard(model) => model.id().into(),
    ModelType::Foreign(_) => None,
    _ => None,
  };
  let model_delta = models
    .get_or_insert_with(model_id.unwrap(), || {
      build_model_gpu(&source.model, ctx).unwrap()
    })
    .create_render_component_delta_stream()
    .into();

  let instance = ReactiveSceneModelGPU {
    node_id: source.node.id(),
    model_id,
    model_delta,
  };

  let state = RenderComponentCell::new(instance);

  source
    .unbound_listen_by(all_delta)
    .fold_signal_flatten(state, |v, state| match v {
      SceneModelImplDelta::model(model) => match model {
        ModelType::Standard(_) => {
          //
          RenderComponentDeltaFlag::ContentRef
        }
        _ => todo!(),
      },
      SceneModelImplDelta::node(node) => {
        //
        RenderComponentDeltaFlag::ContentRef
      }
    })
}
