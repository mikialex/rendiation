use crate::*;

impl SceneRenderable for SceneModel {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    self.visit(|model| model.render(pass, dispatcher, camera, scene))
  }
}

impl SceneNodeControlled for SceneModel {
  fn visit_node(&self, visitor: &mut dyn FnMut(&SceneNode)) {
    self.visit(|model| visitor(&model.node))
  }
}

pub fn setup_pass_core(
  model_input: &SceneModelImpl,
  pass: &mut FrameRenderPass,
  camera: &SceneCamera,
  override_node: Option<&NodeGPU>,
  dispatcher: &dyn RenderComponentAny,
  resources: &SceneRenderResourceGroup,
) {
  match &model_input.model {
    ModelType::Standard(model) => {
      let model = model.read();
      let pass_gpu = dispatcher;

      let cameras = resources.scene_resources.cameras.read().unwrap();
      let camera_gpu = cameras.get_camera_gpu(camera).unwrap();

      let net_visible = resources.node_derives.get_net_visible(&model_input.node);
      if !net_visible {
        return;
      }

      let node_gpu = override_node.unwrap_or(
        resources
          .scene_resources
          .nodes
          .get_node_gpu(&model_input.node)
          .unwrap(),
      );

      let mut materials = resources.resources.model_ctx.materials.write().unwrap();
      let material_gpu = materials.get_or_insert_with(model.material.guid().unwrap(), || {
        model
          .material
          .create_scene_reactive_gpu(&resources.resources.bindable_ctx)
          .unwrap()
      });

      let mut meshes = resources.resources.model_ctx.meshes.write().unwrap();
      if model.mesh.guid().is_none() {
        model.mesh.guid().unwrap();
      }
      let mesh_gpu = meshes.get_or_insert_with(model.mesh.guid().unwrap(), || {
        model
          .mesh
          .create_scene_reactive_gpu(&resources.resources.bindable_ctx)
          .unwrap()
      });

      let draw_command = mesh_gpu.draw_command(model.group);

      dispatch_model_draw_with_preferred_binding_frequency(
        pass_gpu,
        mesh_gpu,
        node_gpu,
        camera_gpu,
        material_gpu,
        draw_command,
        &mut pass.ctx,
      );
    }
    ModelType::Foreign(_) => {
      todo!()
      // if let Some(model) = model.downcast_ref::<Box<dyn SceneRenderable>>() {
      //   model.render(pass, dispatcher, camera, resources)
      // }
    }
    _ => {}
  };
}

pub fn dispatch_model_draw_with_preferred_binding_frequency(
  base: &dyn RenderComponentAny,
  mesh: &MeshGPUInstance,
  node: &NodeGPU,
  camera: &CameraGPU,
  material: &MaterialGPUInstance,
  draw_command: DrawCommand,
  pass: &mut GPURenderPassCtx,
) {
  let components: [&dyn RenderComponentAny; 5] = [
    &base.assign_binding_index(0),
    &mesh.assign_binding_index(2),
    &node.assign_binding_index(2),
    &camera.assign_binding_index(1),
    &material.assign_binding_index(2),
  ];

  RenderEmitter::new(components.as_slice()).render(pass, draw_command);
}

impl SceneRenderable for SceneModelImpl {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    setup_pass_core(self, pass, camera, None, dispatcher, scene);
  }
}

#[pin_project::pin_project]
pub struct StandardModelGPU {
  pub(crate) material_id: Option<usize>,
  material_delta: Option<ReactiveMaterialRenderComponentDeltaSource>,
  pub(crate) mesh_id: Option<usize>,
  mesh_delta: Option<ReactiveMeshRenderComponentDeltaSource>,
  pub(crate) group: MeshDrawGroup,
}

impl Stream for StandardModelGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    let a = this.material_delta.as_mut().map(|v| v.poll_next_unpin(cx));
    let b = this.mesh_delta.as_mut().map(|v| v.poll_next_unpin(cx));
    a.p_or(b)
  }
}

pub type ReactiveStandardModelGPU = impl AsRef<RenderComponentCell<StandardModelGPU>>
  + Stream<Item = RenderComponentDeltaFlag>
  + Unpin;

pub fn build_standard_model_gpu(
  source: &SharedIncrementalSignal<StandardModel>,
  ctx: &GPUModelResourceCtx,
) -> ReactiveStandardModelGPU {
  let s = source.read();
  let gpu = StandardModelGPU {
    material_id: s.material.guid(),
    material_delta: ctx.get_or_create_reactive_material_render_component_delta_source(&s.material),
    mesh_id: s.mesh.guid(),
    mesh_delta: ctx.get_or_create_reactive_mesh_render_component_delta_source(&s.mesh),
    group: s.group,
  };

  let state = RenderComponentCell::new(gpu);
  let ctx = ctx.clone();

  source
    .unbound_listen_by(all_delta_no_init)
    .fold_signal_flatten(state, move |delta, state| {
      match delta {
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
        StandardModelDelta::group(g) => {
          state.group = g;
          RenderComponentDeltaFlag::Draw
        }
        StandardModelDelta::skeleton(_) => RenderComponentDeltaFlag::all(),
      }
      .into()
    })
}

#[allow(clippy::large_enum_variant)]
#[pin_project::pin_project(project = ReactiveSceneModelGPUTypeProj)]
pub enum ReactiveModelGPUType {
  Standard(ReactiveStandardModelGPU),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
}

impl ReactiveModelGPUType {
  pub fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    match self {
      ReactiveModelGPUType::Standard(m) => {
        Box::pin(m.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      ReactiveModelGPUType::Foreign(m) => m
        .as_reactive_component()
        .create_render_component_delta_stream(),
    }
  }
}

impl Stream for ReactiveModelGPUType {
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
  pub(crate) node_id: usize, // todo add stream here
  pub(crate) model_id: Option<usize>,
  model_delta: Option<ReactiveModelRenderComponentDeltaSource>,
}

impl Stream for ReactiveSceneModelGPU {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this
      .model_delta
      .as_mut()
      .map(|v| v.poll_next_unpin(cx))
      .unwrap_or(Poll::Pending)
  }
}

pub trait WebGPUSceneModel: Send + Sync {
  fn create_scene_reactive_gpu(&self, ctx: &GPUModelResourceCtx) -> Option<ReactiveModelGPUType>;
}
define_dyn_trait_downcaster_static!(WebGPUSceneModel);
pub fn register_webgpu_model_features<T>()
where
  T: AsRef<dyn WebGPUSceneModel> + AsMut<dyn WebGPUSceneModel> + 'static,
{
  get_dyn_trait_downcaster_static!(WebGPUSceneModel).register::<T>()
}

impl WebGPUSceneModel for ModelType {
  fn create_scene_reactive_gpu(&self, ctx: &GPUModelResourceCtx) -> Option<ReactiveModelGPUType> {
    match self {
      Self::Standard(model) => ReactiveModelGPUType::Standard(build_standard_model_gpu(model, ctx)),
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(WebGPUSceneModel)
        .downcast_ref(m.as_ref().as_any())?
        .create_scene_reactive_gpu(ctx)?,
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
    node_id: source.node.raw_handle().index(),
    model_id,
    model_delta,
  };

  let state = RenderComponentCell::new(instance);
  let ctx = ctx.clone();

  source
    .unbound_listen_by(all_delta)
    .fold_signal_flatten(state, move |v, state| {
      match v {
        SceneModelImplDelta::model(model) => {
          let model_id = model.guid();
          let model_delta = ctx.get_or_create_reactive_model_render_component_delta_source(&model);
          state.inner.model_id = model_id;
          state.inner.model_delta = model_delta;
          RenderComponentDeltaFlag::ContentRef
        }
        SceneModelImplDelta::node(node) => {
          state.inner.node_id = node.raw_handle().index();
          // todo, handle node change
          RenderComponentDeltaFlag::ContentRef
        }
      }
      .into()
    })
}
