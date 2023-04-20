use __core::task::Poll;

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
    SceneModelType::Standard(model) => {
      let model = model.read();
      let gpu = pass.ctx.gpu;
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

      let material_gpu = model.material.check_update_gpu(
        &mut resources.scene.materials,
        &mut resources.content,
        gpu,
      );

      let mesh_gpu = model.mesh.check_update_gpu(
        &mut resources.scene.meshes,
        &mut resources.custom_storage,
        gpu,
      );

      let components = [pass_gpu, mesh_gpu, node_gpu, camera_gpu, material_gpu];

      let mesh: &dyn MeshDrawcallEmitter = &model.mesh;
      let emitter = MeshDrawcallEmitterWrap {
        group: model.group,
        mesh,
      };

      RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &emitter);
    }
    SceneModelType::Foreign(model) => {
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
      SceneModelType::Standard(model) => model.read().material.is_transparent(),
      SceneModelType::Foreign(model) => {
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
    SceneModelType::Standard(model) => {
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
    SceneModelType::Foreign(model) => {
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

pub struct StandardModelGPU {
  material_id: usize,
  // mesh_id: usize,
}

// impl RenderComponent for StandardModelGPU{
//   //
// }

type ModelGPUReactiveInner = RenderComponentReactive<StandardModelGPU, StandardModelReactive>;
pub type ModelGPUReactive =
  impl AsRef<RenderComponentCell<ModelGPUReactiveInner>> + Stream<Item = RenderComponentDelta>;

pub type ModelRenderComponentReactive = ReactiveRenderComponent<ModelGPUReactiveInner>;

pub fn build_standard_model_gpu(
  source: &SceneItemRef<StandardModel>,
  ctx: &GPUModelResourceCtx,
) -> ModelGPUReactive {
  let s = source.read();
  let gpu = StandardModelGPU {
    material_id: 0,
    // mesh_id: 0,
  };

  let reactive = StandardModelReactive {
    material: ctx.get_or_create_reactive_material_gpu(&s.material),
  };

  let state = RenderComponentReactive::new(gpu, reactive);
  let state = RenderComponentCell::new(state);

  let ctx = ctx.clone();

  source.listen_by(all_delta).fold_signal_flatten(
    state,
    move |delta, state: &mut RenderComponentCell<ModelGPUReactiveInner>| match delta {
      StandardModelDelta::material(material) => {
        let id: usize = 0;
        let delta = ctx.get_or_create_reactive_material_gpu(&material);
        state.inner.gpu.material_id = id;
        state.inner.reactive.material = delta;
        RenderComponentDelta::ContentRef
      }
      StandardModelDelta::mesh(_) => todo!(),
      StandardModelDelta::group(_) => todo!(),
      StandardModelDelta::skeleton(_) => todo!(),
    },
  )
}

#[pin_project::pin_project]
pub struct StandardModelReactive {
  material: MaterialReactive,
  // mesh:
}

impl Stream for StandardModelReactive {
  type Item = RenderComponentDelta;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    let this = self.project();
    early_return_ready!(this.material.poll_next_unpin(cx));
    Poll::Pending
  }
}

pub struct SceneModelGPUInstance {
  node_id: usize,
  model_id: Option<usize>,
}

#[pin_project::pin_project]
pub struct SceneModelGPUReactiveInstance {
  model: Option<ModelRenderComponentReactive>,
  // node: impl Stream<Item = RenderComponentDelta>,
}

// pub enum SceneModelGPUReactive {
//   Standard(ModelGPUReactive),
//   Foreign(Arc<dyn Any + Send + Sync>),
// }

impl Stream for SceneModelGPUReactiveInstance {
  type Item = RenderComponentDelta;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    let mut this = self.project();
    early_return_option_ready!(this.model, cx);
    Poll::Pending
  }
}

type SceneModelGPUReactiveInner =
  RenderComponentReactive<SceneModelGPUInstance, SceneModelGPUReactiveInstance>;
pub type SceneModelGPUReactive =
  impl AsRef<RenderComponentCell<SceneModelGPUReactiveInner>> + Stream<Item = RenderComponentDelta>;

// pub type SceneModelReactive = impl Stream<Item = RenderComponentDelta>;

pub fn build_scene_model_gpu(
  source: &SceneModel,
  ctx: &GPUModelResourceCtx,
  models: &mut StreamMap<ModelGPUReactive>,
) -> SceneModelGPUReactive {
  let source = source.read();
  let model_component_delta_s = match &source.model {
    SceneModelType::Standard(model) => models
      .get_or_insert_with(model.id(), || build_standard_model_gpu(model, ctx))
      .as_ref()
      .create_render_component_delta_stream()
      .into(),
    _ => None,
  };

  let model_id = match &source.model {
    SceneModelType::Standard(model) => model.id().into(),
    SceneModelType::Foreign(_) => None,
    _ => None,
  };

  let reactive = SceneModelGPUReactiveInstance {
    model: model_component_delta_s,
  };

  let instance = SceneModelGPUInstance {
    node_id: source.node.id(),
    model_id,
  };

  let state: SceneModelGPUReactiveInner = RenderComponentReactive::new(instance, reactive);
  let state = RenderComponentCell::new(state);

  source.listen_by(all_delta).fold_signal_flatten(
    state,
    |v, state: &mut RenderComponentCell<SceneModelGPUReactiveInner>| match v {
      SceneModelImplDelta::model(model) => match model {
        SceneModelType::Standard(_) => {
          //
          RenderComponentDelta::ContentRef
        }
        _ => todo!(),
      },
      SceneModelImplDelta::node(node) => {
        //
        RenderComponentDelta::ContentRef
      }
    },
  )
}
