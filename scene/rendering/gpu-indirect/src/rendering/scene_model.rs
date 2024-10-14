use rendering::shape;

use crate::*;

pub struct IndirectPreferredComOrderRendererProvider {
  pub node: Box<dyn RenderImplProvider<Box<dyn IndirectNodeRenderImpl>>>,
  pub model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelRenderImpl>>>>,
}

impl RenderImplProvider<Box<dyn SceneModelRenderer>> for IndirectPreferredComOrderRendererProvider {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.node.register_resource(source, cx);
    self
      .model_impl
      .iter_mut()
      .for_each(|i| i.register_resource(source, cx));
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneModelRenderer> {
    Box::new(IndirectPreferredComOrderRenderer {
      model_impl: self.model_impl.iter().map(|i| i.create_impl(res)).collect(),
      node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
      node_render: self.node.create_impl(res),
    })
  }
}

pub struct IndirectPreferredComOrderRenderer {
  model_impl: Vec<Box<dyn IndirectModelRenderImpl>>,
  node_render: Box<dyn IndirectNodeRenderImpl>,
  node: ForeignKeyReadView<SceneModelRefNode>,
}

impl SceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn make_component<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &'a (dyn CameraRenderImpl + 'a),
    pass: &'a (dyn RenderComponent + 'a),
    tex: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    let node = self.node.get(idx)?;
    let node = self.node_render.make_component_indirect(node)?;

    let camera = camera_gpu.make_component(camera)?;

    let shape = self.model_impl.shape_renderable_indirect(idx)?;
    let material = self.model_impl.material_renderable_indirect(idx, tex)?;

    let pass = Box::new(pass) as Box<dyn RenderComponent + 'a>;

    let contents: [BindingController<Box<dyn RenderComponent + 'a>>; 5] = [
      pass.into_assign_binding_index(0),
      shape.into_assign_binding_index(2),
      node.into_assign_binding_index(2),
      camera.into_assign_binding_index(1),
      material.into_assign_binding_index(2),
    ];

    let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;
    Some((render, todo!()))
  }

  fn render_reorderable_models_impl(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    camera: EntityHandle<SceneCameraEntity>,
    camera_gpu: &dyn CameraRenderImpl,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) {
    // todo, host side prepared multi draw for better performance
    todo!()
  }
}

pub trait IndirectBatchSceneModelRenderer: SceneModelRenderer {
  fn render_batch_models(
    &self,
    models: StorageBufferReadOnlyDataView<[u32]>,
    camera: EntityHandle<SceneCameraEntity>,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut FrameCtx,
  );
}

impl IndirectBatchSceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn render_batch_models(
    &self,
    models: StorageBufferReadOnlyDataView<[u32]>,
    camera: EntityHandle<SceneCameraEntity>,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut FrameCtx,
  ) {
    todo!()
  }
}
