use rendiation_webgpu_midc_downgrade::*;

use crate::*;

impl IndirectSceneRenderer {
  pub fn process_host_driven_indirect_draws<'a>(
    &'a self,
    batch: &dyn HostRenderBatch,
    ctx: &mut FrameCtx,
    camera: &'a dyn RenderComponent,
    pass: &'a dyn RenderComponent,
  ) -> Box<dyn PassContent + 'a> {
    let classifier = self.classify_draws(&mut batch.iter_scene_models());

    let content = classifier
      .values()
      .map(|list| {
        let one_id = *list.first().unwrap();

        let draw_cmd_builder = self.renderer.make_draw_command_builder(one_id).unwrap();

        let first_cmd = draw_cmd_builder.draw_command_host_access(one_id);

        let batch = match first_cmd {
          DrawCommand::Indexed { .. } => {
            let cmds = list
              .iter()
              .map(|id| {
                let cmd = draw_cmd_builder.draw_command_host_access(*id);
                if let DrawCommand::Indexed {
                  base_vertex,
                  indices,
                  instances,
                } = cmd
                {
                  DrawIndexedIndirectArgsStorage::new(
                    indices.len() as u32,
                    instances.len() as u32,
                    indices.start,
                    base_vertex,
                    id.alloc_index(),
                  )
                } else {
                  unreachable!()
                }
              })
              .collect();

            HostDrawCommands::Indexed(cmds)
          }
          DrawCommand::Array { .. } => {
            let cmds = list
              .iter()
              .map(|id| {
                let cmd = draw_cmd_builder.draw_command_host_access(*id);
                if let DrawCommand::Array {
                  instances,
                  vertices,
                } = cmd
                {
                  DrawIndirectArgsStorage::new(
                    vertices.len() as u32,
                    instances.len() as u32,
                    vertices.start,
                    id.alloc_index(),
                  )
                } else {
                  unreachable!()
                }
              })
              .collect();

            HostDrawCommands::NoneIndexed(cmds)
          }
          _ => unreachable!(),
        };

        let (helper, cmd) =
          rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count_host_driven(
            batch, ctx.gpu,
          );

        let provider = HostDrivenIndirectProvider { helper, cmd };

        (Box::new(provider) as Box<dyn IndirectDrawProvider>, one_id)
      })
      .collect();

    Box::new(IndirectScenePassContent {
      renderer: self,
      content,
      pass,
      camera,
      reversed_depth: self.reversed_depth,
    })
  }
}

struct HostDrivenIndirectProvider {
  helper: rendiation_webgpu_midc_downgrade::DowngradeMultiIndirectDrawCountHelper,
  cmd: DrawCommand,
}

impl ShaderHashProvider for HostDrivenIndirectProvider {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.helper.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for HostDrivenIndirectProvider {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.helper.bind(&mut ctx.binding);
  }
}

impl IndirectDrawProvider for HostDrivenIndirectProvider {
  fn create_indirect_invocation_source(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn IndirectBatchInvocationSource> {
    Box::new(HostDrivenIndirectProviderInv {
      helper: self.helper.build(binding),
    })
  }

  fn draw_command(&self) -> DrawCommand {
    self.cmd.clone()
  }
}

struct HostDrivenIndirectProviderInv {
  helper: DowngradeMultiIndirectDrawCountHelperInvocation,
}

impl IndirectBatchInvocationSource for HostDrivenIndirectProviderInv {
  fn current_invocation_scene_model_id(&self, builder: &mut ShaderVertexBuilder) -> Node<u32> {
    self.helper.current_invocation_scene_model_id(builder)
  }
}
