use rendiation_webgpu_reactive_utils::*;

use crate::*;

pub struct Viewer3dRenderingCx<'a> {
  memory: usize,
  dyn_cx: &'a DynCx,
  pub stage: Viewer3dRenderingCxStage<'a>,
  gpu: &'a GPU,
}

impl<'a> Viewer3dRenderingCx<'a> {
  pub fn use_plain_state<T>(&mut self) -> (&mut Self, &mut T) {
    todo!()
  }
  pub fn use_plain_state_init<T>(&mut self, init: &T) -> (&mut Self, &mut T) {
    todo!()
  }
  pub fn use_plain_state_init_by<T>(&mut self, init: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    todo!()
  }

  pub fn use_gpu_state<T>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T) {
    todo!()
  }

  pub fn on_render<R>(
    &mut self,
    f: impl FnOnce(&mut FrameCtx, &Viewer3dSceneCtx) -> R,
  ) -> Option<R> {
    None
  }

  pub fn on_gui<R>(&mut self, f: impl FnOnce(&'a egui::Context) -> R) -> Option<R> {
    None
  }

  pub fn access_query_gpu_cx(&mut self, f: impl FnOnce(&mut QueryGPUHookCx)) {
    let stage = match &mut self.stage {
      Viewer3dRenderingCxStage::Init {} => QueryHookStage::Init { cx: todo!() },
      Viewer3dRenderingCxStage::Uninit {} => QueryHookStage::Unit { cx: todo!() },
      Viewer3dRenderingCxStage::Render { .. } => QueryHookStage::Render,
      Viewer3dRenderingCxStage::Gui { .. } => QueryHookStage::Nothing,
    };
    f(&mut QueryGPUHookCx {
      memory: todo!(),
      dyn_cx: todo!(),
      gpu: todo!(),
      stage,
    });
  }
}

pub enum Viewer3dRenderingCxStage<'a> {
  Init {},
  Uninit {},
  Render {
    target: RenderTargetView,
    content: &'a Viewer3dSceneCtx,
    frame_cx: FrameCtx<'a>,
  },
  Gui {
    context: &'a egui::Context,
  },
}
