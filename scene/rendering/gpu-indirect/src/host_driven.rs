use crate::*;

impl IndirectSceneRenderer {
  pub fn process_host_driven_indirect_draws<'a>(
    &'a self,
    batch: &dyn HostRenderBatch,
    ctx: &mut FrameCtx,
  ) -> Box<dyn SceneRendererPassContentSource + 'a> {
    let classifier = self.classify_draws(&mut batch.iter_scene_models());

    let content = classifier
      .values()
      .filter_map(|list| {
        let (one_id, first_cmd, draw_cmd_builder) = list.iter().find_map(|id| {
          let draw_cmd_builder = self.renderer.make_draw_command_builder(*id);
          self
            .model_error_state
            .report_and_filter_error(*id, &draw_cmd_builder);
          let draw_cmd_builder = draw_cmd_builder.ok()?;
          let first_cmd = draw_cmd_builder.draw_command_host_access(*id);
          first_cmd.map(|c| (id, c, draw_cmd_builder))
        })?;

        let batch = match first_cmd {
          DrawCommand::Indexed { .. } => {
            let cmds = list
              .iter()
              .filter_map(|id| {
                let cmd = draw_cmd_builder.draw_command_host_access(*id)?;
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
                  .into()
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
              .filter_map(|id| {
                let cmd = draw_cmd_builder.draw_command_host_access(*id)?;
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
                  .into()
                } else {
                  unreachable!()
                }
              })
              .collect();

            HostDrawCommands::NoneIndexed(cmds)
          }
          _ => unreachable!(),
        };

        // the buffers are written once after creation, so the host backup is not required
        let alloc = rendiation_webgpu_texture_as_buffer::TextureAsStorageAllocator::new(ctx.gpu);
        let (helper, cmd) =
          rendiation_webgpu_midc_downgrade::downgrade_multi_indirect_draw_count_host_driven(
            batch, ctx.gpu, &alloc,
          );

        let provider = MIDCDowngradeBatch { helper, cmd };

        (Box::new(provider) as Box<dyn IndirectDrawProvider>, *one_id).into()
      })
      .collect();

    Box::new(IndirectScenePassContentSource {
      renderer: self,
      content,
    })
  }
}
