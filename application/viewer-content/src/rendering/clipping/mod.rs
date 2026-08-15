pub use rendiation_csg_clip::*;
pub use rendiation_plane_array_clip::*;

use crate::*;
mod is_solid_filter;
mod plane_array_fill_surface;
pub use is_solid_filter::*;
pub use plane_array_fill_surface::*;

pub struct ViewerClippingRenderer {
  pub csg: CSGClippingRenderer,
  pub plane_array: ClippingPlaneArrayRenderer,
  pub filter: IsSolidFilter,
  pub use_array_clip: bool,
}

pub enum ClipFillType<'a> {
  Forward {
    scene_result: &'a RenderTargetView,
    forward_lighting: &'a dyn RenderComponent,
  },
  Defer(&'a FrameGeneralMaterialBuffer),
}

impl ViewerClippingRenderer {
  pub fn fill_face(&self, scene: EntityHandle<SceneEntity>) -> bool {
    if self.use_array_clip {
      self.plane_array.fill_face(scene)
    } else {
      self.csg.fill_face(scene)
    }
  }

  // if return None, then clip is not enabled
  pub fn use_get_scene_clipping<'a>(
    &'a self,
    scene_id: EntityHandle<SceneEntity>,
    ctx: &mut FrameCtx,
    reverse_z: bool,
  ) -> (
    Option<Box<dyn RenderComponent + 'a>>,
    Option<CSGClippingHelper>,
  ) {
    if self.use_array_clip {
      let render = self.plane_array.use_get_scene_clipping(scene_id, ctx);
      let helper = self.fill_face(scene_id).then_some(CSGClippingHelper(None));
      (render, helper)
    } else {
      self.csg.use_get_scene_clipping(scene_id, ctx, reverse_z)
    }
  }

  // todo we should move this to upstream clipping crate
  //
  // todo this draw should be called after transparent draw.
  // if we want the cap face take effect in occlusion culling, we should
  // distinguish the opaque and transparent part of it.
  pub fn use_draw_surface(
    &self,
    frame_ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    g_buffer: &FrameGeometryBuffer,
    fill_depth_info: CSGClippingHelper,
    target: ClipFillType,
    camera_gpu: &CameraGPU,
    camera: EntityHandle<SceneCameraEntity>,
    scene: EntityHandle<SceneEntity>,
    lighting_sys: &SceneLightSystem,
  ) {
    if self.use_array_clip {
      use_fill_surface(
        &self.plane_array,
        frame_ctx,
        renderer,
        g_buffer,
        target,
        camera,
        camera_gpu,
        scene,
        lighting_sys,
        &self.filter,
      );
    } else {
      let fill_depth = self.csg.draw_csg_fill_surface(
        frame_ctx,
        &g_buffer.normal.expect_texture_view(),
        &g_buffer.depth.expect_texture_view(),
        fill_depth_info.0.unwrap(),
        camera_gpu,
        scene,
        renderer.reversed_depth,
      );

      if let Some(fill_depth) = fill_depth {
        match target {
          ClipFillType::Forward {
            forward_lighting,
            scene_result,
          } => {
            let mut pass = pass("csg fill surface direct forward shading");
            let color_writer =
              DefaultDisplayWriter::extend_pass_desc(&mut pass, scene_result, load_and_store());
            let g_buffer_base_writer = g_buffer.extend_pass_desc_for_subsequent_draw(&mut pass);
            let draw = ForwardCsgSurfaceDraw {
              filled_depth: fill_depth.expect_texture_view(),
              reverse_z: renderer.reversed_depth,
              camera: camera_gpu.clone(),
            };
            let mut draw = RenderArray([
              &color_writer as &dyn RenderComponent,
              &g_buffer_base_writer as &dyn RenderComponent,
              forward_lighting,
              &draw,
            ])
            .draw_quad();
            pass.render_ctx(frame_ctx).by(&mut draw);
          }
          ClipFillType::Defer(_) => todo!(),
        }
      }
    }
  }
}
