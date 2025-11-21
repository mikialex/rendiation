mod defer_protocol;
pub use defer_protocol::*;
use rendiation_oit::draw_weighted_oit;
use rendiation_texture_gpu_process::ToneMap;

use crate::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LightingTechniqueKind {
  Forward,
  DeferLighting,
  // Visibility,
}

pub struct LightingRenderingCx<'a> {
  pub lighting: SceneLightSystem<'a>,
  pub tonemap: &'a ToneMap,
  pub deferred_mat_supports: &'a DeferLightingMaterialRegistry,
  pub lighting_method: LightingTechniqueKind,
}

// todo，fix transparent rendering in defer mode
pub fn render_lighting_scene_content(
  ctx: &mut FrameCtx,
  lighting_cx: &LightingRenderingCx,
  cull_cx: &mut ViewerCulling,
  renderer: &ViewerSceneRenderer,
  scene: EntityHandle<SceneEntity>,
  viewport: &ViewerViewPort,
  scene_result: &RenderTargetView,
  g_buffer: &FrameGeometryBuffer,
  only_draw_g_buffer: bool,
) {
  let camera = viewport.camera;
  let camera_gpu = renderer.cameras.make_component(camera).unwrap();
  let camera_gpu = &camera_gpu;

  let (color_ops, depth_ops) = renderer
    .background
    .init_clear(scene, renderer.reversed_depth);

  let mut background = renderer
    .background
    .draw(scene, camera_gpu, lighting_cx.tonemap);

  match lighting_cx.lighting_method {
    LightingTechniqueKind::Forward => {
      ctx.scope(|ctx| {
        let lighting = lighting_cx
          .lighting
          .get_scene_forward_lighting_component(scene, camera);

        let all_opaque_object = renderer.batch_extractor.extract_scene_batch(
          scene,
          SceneContentKey::only_opaque_objects(),
          renderer.scene,
        );

        let all_transparent_object = renderer.batch_extractor.extract_scene_batch(
          scene,
          SceneContentKey::only_alpha_blend_objects(),
          renderer.scene,
        );

        let mut all_transparent_object =
          if let SceneModelRenderBatch::Host(all_transparent_object) = all_transparent_object {
            if renderer.oit.should_reorder_draw_list() {
              let camera_position = renderer
                .camera_transforms
                .access(&camera)
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

        cull_cx.install_frustum_culler(&mut all_transparent_object, camera_gpu, camera);

        match renderer.oit.clone() {
          ViewerTransparentRenderer::NaiveAlphaBlend => {
            ctx.scope(|ctx| {
              let mut pass_base = pass("scene forward all");
              let color_writer =
                DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, color_ops);

              let skip_entity_id = !ctx
                .gpu
                .info()
                .downgrade_info
                .flags
                .contains(DownlevelFlags::INDEPENDENT_BLEND); // to support webgl!

              let g_buffer_base_writer =
                g_buffer.extend_pass_desc(&mut pass_base, depth_ops, skip_entity_id);

              let scene_pass_dispatcher = &RenderArray([
                &color_writer as &dyn RenderComponent,
                &g_buffer_base_writer as &dyn RenderComponent,
                lighting.as_ref(),
              ]) as &dyn RenderComponent;

              let mut all_transparent_object = renderer.scene.make_scene_batch_pass_content(
                all_transparent_object,
                camera_gpu,
                scene_pass_dispatcher,
                ctx,
              );

              cull_cx
                .draw_with_oc_maybe_enabled(
                  ctx,
                  renderer,
                  scene_pass_dispatcher,
                  camera_gpu,
                  viewport,
                  &mut |pass| pass.by(&mut background),
                  pass_base,
                  all_opaque_object,
                )
                .by(&mut all_transparent_object);
            });
          }
          ViewerTransparentRenderer::Loop32OIT(oit) => {
            ctx.scope(|ctx| {
              let mut pass_base_for_opaque = pass("scene forward opaque");

              let g_buffer_base_writer =
                g_buffer.extend_pass_desc(&mut pass_base_for_opaque, depth_ops, false);
              let color_writer = DefaultDisplayWriter::extend_pass_desc(
                &mut pass_base_for_opaque,
                scene_result,
                color_ops,
              );

              let scene_pass_dispatcher = &RenderArray([
                &color_writer as &dyn RenderComponent,
                &g_buffer_base_writer as &dyn RenderComponent,
                lighting.as_ref(),
              ]) as &dyn RenderComponent;

              cull_cx.draw_with_oc_maybe_enabled(
                ctx,
                renderer,
                scene_pass_dispatcher,
                camera_gpu,
                viewport,
                &mut |pass| pass.by(&mut background),
                pass_base_for_opaque,
                all_opaque_object,
              );

              let mut pass_base_transparent = pass("scene forward transparent");
              let g_buffer_base_writer =
                g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base_transparent);

              let scene_pass_dispatcher = &RenderArray([
                &g_buffer_base_writer as &dyn RenderComponent,
                lighting.as_ref(),
              ]) as &dyn RenderComponent;

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
              let mut pass_base_for_opaque = pass("scene forward opaque");

              let g_buffer_base_writer =
                g_buffer.extend_pass_desc(&mut pass_base_for_opaque, depth_ops, false);

              let color_writer = DefaultDisplayWriter::extend_pass_desc(
                &mut pass_base_for_opaque,
                scene_result,
                color_ops,
              );

              let scene_pass_dispatcher = &RenderArray([
                &color_writer as &dyn RenderComponent,
                &g_buffer_base_writer as &dyn RenderComponent,
                lighting.as_ref(),
              ]) as &dyn RenderComponent;

              cull_cx.draw_with_oc_maybe_enabled(
                ctx,
                renderer,
                scene_pass_dispatcher,
                camera_gpu,
                viewport,
                &mut |pass| pass.by(&mut background),
                pass_base_for_opaque,
                all_opaque_object,
              );

              let mut pass_base_transparent = pass("scene forward transparent");
              let g_buffer_base_writer =
                g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base_transparent);

              let scene_pass_dispatcher = &RenderArray([
                &g_buffer_base_writer as &dyn RenderComponent,
                lighting.as_ref(),
              ]) as &dyn RenderComponent;

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
        }
      })
    }
    LightingTechniqueKind::DeferLighting => {
      ctx.scope(|ctx| {
        let mut pass_base = pass("scene defer encode");

        let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops, false);
        let mut m_buffer = FrameGeneralMaterialBuffer::new(ctx);

        let indices = m_buffer.extend_pass_desc(&mut pass_base);
        let material_writer = FrameGeneralMaterialBufferEncoder {
          indices,
          materials: lighting_cx.deferred_mat_supports,
        };

        let scene_pass_dispatcher = &RenderArray([
          &g_buffer_base_writer as &dyn RenderComponent,
          &material_writer,
        ]) as &dyn RenderComponent;

        let main_scene_content = renderer.batch_extractor.extract_scene_batch(
          scene,
          SceneContentKey::default(),
          renderer.scene,
        );

        cull_cx.draw_with_oc_maybe_enabled(
          ctx,
          renderer,
          scene_pass_dispatcher,
          camera_gpu,
          viewport,
          &mut |pass| pass,
          pass_base,
          main_scene_content,
        );

        if !only_draw_g_buffer {
          ctx.scope(|ctx| {
            let geometry_from_g_buffer = Box::new(FrameGeometryBufferReconstructGeometryCtx {
              camera: &camera_gpu,
              g_buffer,
            }) as Box<dyn GeometryCtxProvider>;
            let surface_from_m_buffer = Box::new(FrameGeneralMaterialBufferReconstructSurface {
              m_buffer: &m_buffer,
              registry: lighting_cx.deferred_mat_supports,
            });
            let lighting = lighting_cx.lighting.get_scene_lighting_component(
              scene,
              camera,
              geometry_from_g_buffer,
              surface_from_m_buffer,
            );

            let lighting = RenderArray([
              &DefaultDisplayWriter {
                write_channel_index: 0,
              } as &dyn RenderComponent,
              lighting.as_ref(),
            ]);

            let _ = pass("deferred lighting compute")
              .with_color(scene_result, color_ops)
              .render_ctx(ctx)
              .by(&mut background)
              .by(&mut lighting.draw_quad());
          });
        }
      });
    }
  }
}
