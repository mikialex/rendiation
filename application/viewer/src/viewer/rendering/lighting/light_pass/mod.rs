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
  cull_cx: &ViewerCulling,
  renderer: &ViewerSceneRenderer,
  content: &Viewer3dSceneCtx,
  scene_derive: &Viewer3dSceneDerive,
  scene_result: &RenderTargetView,
  g_buffer: &FrameGeometryBuffer,
) {
  let main_camera_gpu = renderer
    .cameras
    .make_component(content.main_camera)
    .unwrap();
  let main_camera_gpu = &main_camera_gpu;

  let (color_ops, depth_ops) = renderer
    .background
    .init_clear(content.scene, renderer.reversed_depth);

  let mut background =
    renderer
      .background
      .draw(content.scene, main_camera_gpu, lighting_cx.tonemap);

  match lighting_cx.lighting_method {
    LightingTechniqueKind::Forward => {
      let lighting = lighting_cx
        .lighting
        .get_scene_forward_lighting_component(content.scene);

      let all_opaque_object = renderer.batch_extractor.extract_scene_batch(
        content.scene,
        SceneContentKey::only_opaque_objects(),
        renderer.scene,
        ctx,
      );

      let all_transparent_object = renderer.batch_extractor.extract_scene_batch(
        content.scene,
        SceneContentKey::only_alpha_blend_objects(),
        renderer.scene,
        ctx,
      );

      let mut all_transparent_object =
        if let SceneModelRenderBatch::Host(all_transparent_object) = &all_transparent_object {
          let camera_position = scene_derive
            .camera_transforms
            .access(&content.main_camera)
            .unwrap()
            .world
            .position();
          let all_transparent_object = TransparentHostOrderer {
            world_bounding: scene_derive.sm_world_bounding.clone(),
          }
          .reorder_content(all_transparent_object.as_ref(), camera_position);

          SceneModelRenderBatch::Host(all_transparent_object)
        } else {
          all_transparent_object
        };

      cull_cx.install_device_frustum_culler(
        &mut all_transparent_object,
        main_camera_gpu,
        content.main_camera,
      );

      match renderer.oit.clone() {
        ViewerTransparentRenderer::NaiveAlphaBlend => {
          let mut pass_base = pass("scene forward all");
          let color_writer =
            DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, color_ops);
          let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops);

          let scene_pass_dispatcher = &RenderArray([
            &color_writer as &dyn RenderComponent,
            &g_buffer_base_writer as &dyn RenderComponent,
            lighting.as_ref(),
          ]) as &dyn RenderComponent;

          let mut all_transparent_object = renderer.scene.make_scene_batch_pass_content(
            all_transparent_object,
            main_camera_gpu,
            scene_pass_dispatcher,
            ctx,
          );

          cull_cx
            .draw_with_oc_maybe_enabled(
              ctx,
              renderer,
              scene_pass_dispatcher,
              main_camera_gpu,
              content.main_camera,
              &mut |pass| pass.by(&mut background),
              pass_base,
              all_opaque_object,
            )
            .by(&mut all_transparent_object);
        }
        ViewerTransparentRenderer::Loop32OIT(oit) => {
          let mut pass_base_for_opaque = pass("scene forward opaque");

          let g_buffer_base_writer =
            g_buffer.extend_pass_desc(&mut pass_base_for_opaque, depth_ops);
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
            main_camera_gpu,
            content.main_camera,
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
            main_camera_gpu,
            scene_pass_dispatcher,
            renderer.reversed_depth,
          );
        }
        ViewerTransparentRenderer::WeightedOIT => {
          let mut pass_base_for_opaque = pass("scene forward opaque");

          let g_buffer_base_writer =
            g_buffer.extend_pass_desc(&mut pass_base_for_opaque, depth_ops);

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
            main_camera_gpu,
            content.main_camera,
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
            main_camera_gpu,
            scene_pass_dispatcher,
            renderer.reversed_depth,
          );
        }
      }
    }
    LightingTechniqueKind::DeferLighting => {
      let mut pass_base = pass("scene defer encode");

      let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops);
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
        content.scene,
        SceneContentKey::default(),
        renderer.scene,
        ctx,
      );

      cull_cx.draw_with_oc_maybe_enabled(
        ctx,
        renderer,
        scene_pass_dispatcher,
        main_camera_gpu,
        content.main_camera,
        &mut |pass| pass,
        pass_base,
        main_scene_content,
      );

      let geometry_from_g_buffer = Box::new(FrameGeometryBufferReconstructGeometryCtx {
        camera: &main_camera_gpu,
        g_buffer,
      }) as Box<dyn GeometryCtxProvider>;
      let surface_from_m_buffer = Box::new(FrameGeneralMaterialBufferReconstructSurface {
        m_buffer: &m_buffer,
        registry: lighting_cx.deferred_mat_supports,
      });
      let lighting = lighting_cx.lighting.get_scene_lighting_component(
        content.scene,
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
    }
  }
}
