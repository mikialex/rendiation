use rendiation_oit::{draw_weighted_oit, OitLoop32Renderer};

use crate::*;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewerTransparentContentRenderStyle {
  NaiveAlphaBlend,
  Loop32OIT,
  WeightedOIT,
  Opaque,
}

#[derive(Clone)]
pub enum ViewerTransparentRenderer {
  NaiveAlphaBlend,
  Loop32OIT(Arc<RwLock<OitLoop32Renderer>>),
  WeightedOIT,
  Opaque,
}

impl ViewerTransparentRenderer {
  pub fn should_reorder_draw_list(&self) -> bool {
    matches!(self, ViewerTransparentRenderer::NaiveAlphaBlend)
  }

  pub fn render(
    &self,
    ctx: &mut FrameCtx,
    cull_cx: &mut ViewerCulling,
    g_buffer: &FrameGeometryBuffer,
    renderer: &ViewerSceneRenderer,
    all_transparent_object: SceneModelRenderBatch,
    camera_gpu: &CameraGPU,
    viewport: &ViewerViewPort,
    scene_result: &RenderTargetView,
    pass_com: &dyn RenderComponent,
    opaque_pass_dispatcher: &dyn RenderComponent,
    draw_opaque_content: impl FnOnce(&mut FrameCtx<'_>, &mut ViewerCulling) -> Option<ActiveRenderPass>,
  ) {
    let mut all_transparent_object =
      if let SceneModelRenderBatch::Host(all_transparent_object) = all_transparent_object {
        if self.should_reorder_draw_list() {
          let camera_position = renderer
            .camera_transforms
            .access(&viewport.camera)
            .unwrap()
            .world
            .position();
          let all_transparent_object = TransparentHostOrderer {
            world_bounding: renderer.sm_world_bounding.clone(),
          }
          .reorder_content(all_transparent_object.as_ref(), camera_position);
          SceneModelRenderBatch::Host(all_transparent_object)
        } else {
          SceneModelRenderBatch::Host(all_transparent_object)
        }
      } else {
        all_transparent_object
      };

    cull_cx.install_frustum_culler(&mut all_transparent_object, camera_gpu, viewport.camera);

    match self {
      ViewerTransparentRenderer::NaiveAlphaBlend => {
        ctx.scope(|ctx| {
          let mut all_transparent_object_pass_content =
            renderer.scene.make_scene_batch_pass_content(
              all_transparent_object.clone(),
              camera_gpu,
              opaque_pass_dispatcher,
              ctx,
            );

          if let Some(active_pass) = draw_opaque_content(ctx, cull_cx) {
            active_pass.by(&mut all_transparent_object_pass_content);
          } else {
            let mut pass_base = pass("scene forward transparent extra split alpha blend");
            DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, load_and_store());
            g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base);

            pass_base
              .render_ctx(ctx)
              .by(&mut all_transparent_object_pass_content);
          }
        });
      }
      ViewerTransparentRenderer::Loop32OIT(oit) => {
        ctx.scope(|ctx| {
          draw_opaque_content(ctx, cull_cx);

          let mut pass_base_transparent = pass("scene forward transparent loop32 oit");
          let g_buffer_base_writer =
            g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base_transparent);

          let scene_pass_dispatcher =
            &RenderArray([&g_buffer_base_writer as &dyn RenderComponent, pass_com])
              as &dyn RenderComponent;

          let mut oit = oit.write();
          let oit = oit.get_renderer_instance(ctx.frame_size(), ctx.gpu);
          oit.draw_loop32_oit(
            ctx,
            all_transparent_object,
            pass_base_transparent,
            scene_result,
            renderer.scene,
            camera_gpu,
            scene_pass_dispatcher,
            renderer.reversed_depth,
          );
        });
      }
      ViewerTransparentRenderer::WeightedOIT => {
        ctx.scope(|ctx| {
          draw_opaque_content(ctx, cull_cx);

          let mut pass_base_transparent = pass("scene forward transparent weighted oit");
          let g_buffer_base_writer =
            g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base_transparent);

          let scene_pass_dispatcher =
            &RenderArray([&g_buffer_base_writer as &dyn RenderComponent, pass_com])
              as &dyn RenderComponent;

          draw_weighted_oit(
            ctx,
            all_transparent_object,
            pass_base_transparent,
            scene_result,
            renderer.scene,
            camera_gpu,
            scene_pass_dispatcher,
            renderer.reversed_depth,
          );
        });
      }
      ViewerTransparentRenderer::Opaque => {
        draw_opaque_content(ctx, cull_cx);
      }
    }
  }
}

pub struct DisableAllChannelBlend;
impl ShaderHashProvider for DisableAllChannelBlend {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for DisableAllChannelBlend {}
impl GraphicsShaderProvider for DisableAllChannelBlend {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|b, _| {
      for c in &mut b.frag_output {
        c.states.blend = None;
      }
    })
  }
}
