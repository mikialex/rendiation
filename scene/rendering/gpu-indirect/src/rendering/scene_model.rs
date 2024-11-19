use crate::*;

pub trait IndirectBatchSceneModelRenderer: SceneModelRenderer {
  /// the caller must guarantee the batch source can be drawn by the implementation selected by any_id
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()>;

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>>;
}

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

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    self.node.deregister_resource(source);
    self
      .model_impl
      .iter_mut()
      .for_each(|i| i.deregister_resource(source));
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
  fn render_scene_model(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
    tex: &GPUTextureBindingSystem,
  ) -> Result<(), UnableToRenderSceneModelError> {
    let model_id = create_uniform(idx.alloc_index(), &cx.gpu.device);
    let cmd = self
      .make_draw_command_builder(idx)
      .unwrap()
      .draw_command_host_access(idx);

    struct SingleModelImmediateDraw {
      model_id: UniformBufferDataView<u32>,
      cmd: DrawCommand,
    }

    impl ShaderHashProvider for SingleModelImmediateDraw {
      shader_hash_type_id! {}
    }

    impl ShaderPassBuilder for SingleModelImmediateDraw {
      fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.model_id);
      }
    }

    impl IndirectDrawProvider for SingleModelImmediateDraw {
      fn create_indirect_invocation_source(
        &self,
        binding: &mut ShaderBindGroupBuilder,
      ) -> Box<dyn IndirectBatchInvocationSource> {
        struct SingleModelImmediateDrawInvocation {
          model_id: UniformNode<u32>,
        }

        impl IndirectBatchInvocationSource for SingleModelImmediateDrawInvocation {
          fn current_invocation_scene_model_id(&self, _: &ShaderVertexBuilder) -> Node<u32> {
            self.model_id.load()
          }
        }

        Box::new(SingleModelImmediateDrawInvocation {
          model_id: binding.bind_by(&self.model_id.clone()),
        })
      }

      fn draw_command(&self) -> DrawCommand {
        self.cmd.clone()
      }
    }

    self
      .render_indirect_batch_models(
        &SingleModelImmediateDraw { model_id, cmd },
        idx,
        camera,
        tex,
        pass,
        cx,
      )
      .unwrap();

    Ok(())
  }
}

impl IndirectBatchSceneModelRenderer for IndirectPreferredComOrderRenderer {
  fn render_indirect_batch_models(
    &self,
    models: &dyn IndirectDrawProvider,
    any_id: EntityHandle<SceneModelEntity>,
    camera: &dyn RenderComponent,
    tex: &GPUTextureBindingSystem,
    pass: &dyn RenderComponent,
    cx: &mut GPURenderPassCtx,
  ) -> Option<()> {
    let node = self.node.get(any_id)?;
    let node = self.node_render.make_component_indirect(node)?;

    let shape = self.model_impl.shape_renderable_indirect(any_id)?;
    let material = self.model_impl.material_renderable_indirect(any_id, tex)?;

    let camera = Box::new(camera) as Box<dyn RenderComponent>;
    let pass = Box::new(pass) as Box<dyn RenderComponent>;
    let draw_source =
      Box::new(IndirectDrawProviderAsRenderComponent(models)) as Box<dyn RenderComponent>;

    let command = models.draw_command();

    let contents: [BindingController<Box<dyn RenderComponent>>; 6] = [
      draw_source.into_assign_binding_index(0),
      pass.into_assign_binding_index(0),
      shape.into_assign_binding_index(2),
      node.into_assign_binding_index(2),
      camera.into_assign_binding_index(1),
      material.into_assign_binding_index(2),
    ];

    let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;
    render.render(cx, command);
    Some(())
  }

  fn make_draw_command_builder(
    &self,
    any_idx: EntityHandle<SceneModelEntity>,
  ) -> Option<Box<dyn DrawCommandBuilder>> {
    self.model_impl.make_draw_command_builder(any_idx)
  }
}
