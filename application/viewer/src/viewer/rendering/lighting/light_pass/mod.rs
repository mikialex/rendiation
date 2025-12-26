mod defer_protocol;
pub use defer_protocol::*;
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

pub fn render_lighting_scene_content(
  ctx: &mut FrameCtx,
  lighting_cx: &LightingRenderingCx,
  cull_cx: &mut ViewerCulling,
  renderer: &ViewerSceneRenderer,
  clipping: &CSGClippingRenderer,
  clip_component: &dyn RenderComponent,
  scene: EntityHandle<SceneEntity>,
  viewport: &ViewerViewPort,
  scene_result: &RenderTargetView,
  g_buffer: &FrameGeometryBuffer,
  only_draw_g_buffer: bool,
) {
  let required_stencil_ops = clipping.get_stencil_load_op().unwrap_or(load_and_store());

  let camera = viewport.camera;
  let camera_gpu = renderer.cameras.make_component(camera).unwrap();
  let camera_gpu = &camera_gpu;

  let (color_ops, depth_ops) = renderer
    .background
    .init_clear(scene, renderer.reversed_depth);

  let mut background = renderer
    .background
    .draw(scene, camera_gpu, lighting_cx.tonemap);

  let is_opaque_all = matches!(
    renderer.transparent_content_renderer,
    ViewerTransparentRenderer::Opaque
  );
  let all_opaque_object = renderer.batch_extractor.extract_scene_batch(
    scene,
    if is_opaque_all {
      SceneContentKey::default()
    } else {
      SceneContentKey::only_opaque_objects()
    },
    renderer.scene,
  );

  let blend_disabler = if is_opaque_all {
    OptionRender(Some(DisableAllChannelBlend))
  } else {
    OptionRender(None)
  };

  let all_transparent_object = renderer.batch_extractor.extract_scene_batch(
    scene,
    SceneContentKey::only_alpha_blend_objects(),
    renderer.scene,
  );

  // always get forward lighting because we may use it in none forward case
  let forward_lighting = lighting_cx
    .lighting
    .get_scene_forward_lighting_component(scene, camera);
  let pass_com = &RenderArray([forward_lighting.as_ref(), clip_component]);

  match lighting_cx.lighting_method {
    LightingTechniqueKind::Forward => ctx.scope(|ctx| {
      let mut pass_base = pass("scene forward");
      let color_writer =
        DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, color_ops);
      let g_buffer_base_writer =
        g_buffer.extend_pass_desc(&mut pass_base, depth_ops, required_stencil_ops);

      let opaque_scene_pass_dispatcher = &RenderArray([
        &blend_disabler as &dyn RenderComponent,
        &color_writer as &dyn RenderComponent,
        &g_buffer_base_writer as &dyn RenderComponent,
        pass_com,
      ]) as &dyn RenderComponent;

      let draw_opaque = |ctx: &mut FrameCtx<'_>, cull_cx: &mut ViewerCulling| {
        let pass = cull_cx.draw_with_oc_maybe_enabled(
          ctx,
          renderer,
          opaque_scene_pass_dispatcher,
          camera_gpu,
          viewport,
          &mut |pass| pass.by(&mut background),
          pass_base,
          all_opaque_object,
        );

        if !clipping.fill_face(scene) {
          return Some(pass);
        }
        drop(pass);

        clipping.draw_csg_surface(
          ctx,
          g_buffer,
          CSGxClipFillType::Forward {
            scene_result,
            forward_lighting: &forward_lighting,
          },
          camera_gpu,
          scene,
          renderer.reversed_depth,
        );

        None
      };

      renderer.transparent_content_renderer.render(
        ctx,
        cull_cx,
        g_buffer,
        renderer,
        all_transparent_object,
        camera_gpu,
        viewport,
        scene_result,
        pass_com,
        opaque_scene_pass_dispatcher,
        draw_opaque,
      );
    }),
    LightingTechniqueKind::DeferLighting => {
      ctx.scope(|ctx| {
        let mut pass_base = pass("scene defer encode");

        let g_buffer_base_writer =
          g_buffer.extend_pass_desc(&mut pass_base, depth_ops, required_stencil_ops);
        let mut m_buffer = FrameGeneralMaterialBuffer::new(ctx);

        let indices = m_buffer.extend_pass_desc(&mut pass_base);
        let material_writer = FrameGeneralMaterialBufferEncoder {
          indices,
          materials: lighting_cx.deferred_mat_supports,
        };

        let scene_pass_dispatcher = &RenderArray([
          &DisableAllChannelBlend as &dyn RenderComponent,
          &g_buffer_base_writer,
          &material_writer,
          clip_component,
        ]) as &dyn RenderComponent;

        cull_cx.draw_with_oc_maybe_enabled(
          ctx,
          renderer,
          scene_pass_dispatcher,
          camera_gpu,
          viewport,
          &mut |pass| pass,
          pass_base,
          all_opaque_object,
        );

        clipping.draw_csg_surface(
          ctx,
          g_buffer,
          CSGxClipFillType::Defer(&m_buffer),
          camera_gpu,
          scene,
          renderer.reversed_depth,
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

          let mut pass_base = pass("scene forward transparent in defer mode");
          let color_writer =
            DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, load_and_store());
          let g_buffer_base_writer = g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass_base);

          let opaque_scene_pass_dispatcher = &RenderArray([
            &blend_disabler as &dyn RenderComponent,
            &color_writer as &dyn RenderComponent,
            &g_buffer_base_writer as &dyn RenderComponent,
            pass_com,
          ]) as &dyn RenderComponent;

          renderer.transparent_content_renderer.render(
            ctx,
            cull_cx,
            g_buffer,
            renderer,
            all_transparent_object,
            camera_gpu,
            viewport,
            scene_result,
            pass_com,
            opaque_scene_pass_dispatcher,
            |ctx: &mut FrameCtx<'_>, _cull_cx: &mut ViewerCulling| Some(pass_base.render_ctx(ctx)),
          );
        }
      });
    }
  }
}
