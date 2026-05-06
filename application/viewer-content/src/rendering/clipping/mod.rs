use crate::*;

mod clipping_csg;
mod clipping_plane_array;
pub use clipping_csg::*;
pub use clipping_plane_array::*;

pub struct ViewerClippingRenderer {
  pub csg: CSGClippingRenderer,
  pub plane_array: ClippingPlaneArrayRenderer,
  pub use_array_clip: bool,
}

pub enum ClipFillType<'a> {
  Forward {
    scene_result: &'a RenderTargetView,
    forward_lighting: &'a dyn RenderComponent,
  },
  Defer(&'a FrameGeneralMaterialBuffer),
}

#[derive(Clone)]

pub struct ViewerClippingHelper(pub Option<AtomicImageDowngrade>);

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
    Option<ViewerClippingHelper>,
  ) {
    if self.use_array_clip {
      let render = self.plane_array.use_get_scene_clipping(scene_id, ctx);
      let helper = self
        .fill_face(scene_id)
        .then_some(ViewerClippingHelper(None));
      (render, helper)
    } else {
      self.csg.use_get_scene_clipping(scene_id, ctx, reverse_z)
    }
  }

  pub fn use_draw_csg_surface(
    &self,
    frame_ctx: &mut FrameCtx,
    renderer: &ViewerSceneRenderer,
    g_buffer: &FrameGeometryBuffer,
    fill_depth_info: ViewerClippingHelper,
    target: ClipFillType,
    camera_gpu: &CameraGPU,
    scene: EntityHandle<SceneEntity>,
  ) {
    if self.use_array_clip {
      self
        .plane_array
        .use_fill_surface(frame_ctx, renderer, g_buffer, target, camera_gpu, scene);
    } else {
      self.csg.draw_csg_surface(
        frame_ctx,
        g_buffer,
        fill_depth_info.0.unwrap(),
        target,
        camera_gpu,
        scene,
        renderer.reversed_depth,
      );
    }
  }
}
