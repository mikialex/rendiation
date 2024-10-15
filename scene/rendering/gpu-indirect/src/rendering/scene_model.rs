use crate::*;

pub trait IndirectBatchSource: ShaderHashProvider + ShaderPassBuilder {
  fn create_indirect_invocation_source(&self) -> Box<dyn IndirectBatchInvocationSource>;
  fn draw_command(&self) -> DrawCommand;
}

pub trait IndirectBatchInvocationSource {
  fn current_invocation_scene_model_id(&self) -> Node<u32>;
}

pub trait IndirectBatchSceneModelRenderer: SceneModelRenderer {
  /// the caller must guarantee the batch source can be drawn by the implementation selected by any_id
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectBatchSource,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()>;
}

// pub struct IndirectPreferredComOrderRendererProvider {
//   pub node: Box<dyn RenderImplProvider<Box<dyn IndirectNodeRenderImpl>>>,
//   pub model_impl: Vec<Box<dyn RenderImplProvider<Box<dyn IndirectModelRenderImpl>>>>,
// }

// impl RenderImplProvider<Box<dyn SceneModelRenderer>> for IndirectPreferredComOrderRendererProvider {
//   fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
//     self.node.register_resource(source, cx);
//     self
//       .model_impl
//       .iter_mut()
//       .for_each(|i| i.register_resource(source, cx));
//   }

//   fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> Box<dyn SceneModelRenderer> {
//     Box::new(IndirectPreferredComOrderRenderer {
//       model_impl: self.model_impl.iter().map(|i| i.create_impl(res)).collect(),
//       node: global_entity_component_of::<SceneModelRefNode>().read_foreign_key(),
//       node_render: self.node.create_impl(res),
//     })
//   }
// }

// pub struct IndirectPreferredComOrderRenderer {
//   model_impl: Vec<Box<dyn IndirectModelRenderImpl>>,
//   node_render: Box<dyn IndirectNodeRenderImpl>,
//   node: ForeignKeyReadView<SceneModelRefNode>,
// }

// impl SceneModelRenderer for IndirectPreferredComOrderRenderer {
//   // fn render_reorderable_models_impl(
//   //   &self,
//   //   models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
//   //   camera: &dyn RenderComponent,
//   //   pass: &dyn RenderComponent,
//   //   cx: &mut GPURenderPassCtx,
//   //   tex: &GPUTextureBindingSystem,
//   // ) -> bool {
//   //   // todo, host side prepared multi draw for better performance
//   //   todo!()
//   // }

//   fn render_scene_model(
//     &self,
//     idx: EntityHandle<SceneModelEntity>,
//     camera: &dyn RenderComponent,
//     pass: &dyn RenderComponent,
//     cx: &mut GPURenderPassCtx,
//     tex: &GPUTextureBindingSystem,
//   ) -> Option<()> {
//     self.render_indirect_batch_models(todo!(), idx, camera, tex, pass, cx);
//     Some(())
//   }
// }

// impl IndirectBatchSceneModelRenderer for IndirectPreferredComOrderRenderer {
//   fn render_indirect_batch_models(
//     &self,
//     models: &dyn IndirectBatchSource,
//     any_id: EntityHandle<SceneModelEntity>,
//     camera: &dyn RenderComponent,
//     tex: &GPUTextureBindingSystem,
//     pass: &dyn RenderComponent,
//     cx: &mut GPURenderPassCtx,
//   ) -> Option<()> {
//     // let node = self.node.get(any_id)?;
//     // let node = self.node_render.make_component_indirect(node)?;

//     // let shape = self.model_impl.shape_renderable_indirect(any_id)?;
//     // let material = self.model_impl.material_renderable_indirect(any_id, tex)?;

//     // let pass = Box::new(pass) as Box<dyn RenderComponent>;

//     // let contents: [BindingController<Box<dyn RenderComponent>>; 5] = [
//     //   pass.into_assign_binding_index(0),
//     //   shape.into_assign_binding_index(2),
//     //   node.into_assign_binding_index(2),
//     //   camera.into_assign_binding_index(1),
//     //   material.into_assign_binding_index(2),
//     // ];

//     // let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;
//     // Some((render, todo!()))
//     Some(())
//   }
// }
