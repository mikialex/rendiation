use crate::*;

#[derive(Default)]
struct UI3DMemory {
  memory: FunctionMemory,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for UI3DMemory {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    self.memory.cleanup(&mut UI3dBuildCx {
      writer: cx.writer,
      cx: cx.dyn_cx,
      pick_group: cx.pick_group,
    } as *mut _ as *mut ());
  }
}

pub fn widget_root(viewer_cx: &mut ViewerCx, f: impl FnOnce(&mut UI3dCx)) {
  let (viewer_cx, memory) = viewer_cx.use_state_init(|_| UI3DMemory::default());
  let widget_scene = viewer_cx.viewer.scene.widget_scene;
  let mut cx = match &mut viewer_cx.stage {
    ViewerCxStage::EventHandling {
      reader,
      interaction,
      input,
      widget_cx,
      ..
    } => UI3dCx::new_event_stage(
      &mut memory.memory,
      UIEventStageCx {
        platform_event: input,
        interaction_cx: interaction,
        widget_env: *widget_cx,
      },
      reader,
      viewer_cx.dyn_cx,
      &mut viewer_cx.viewer.intersection_group,
    ),
    ViewerCxStage::SceneContentUpdate { writer } => UI3dCx::new_update_stage(
      &mut memory.memory,
      viewer_cx.dyn_cx,
      writer,
      &mut viewer_cx.viewer.intersection_group,
    ),
  };

  if cx.is_creating() && cx.event.is_some() {
    // skip the first event stage when first time init
    return;
  }

  let mut scene_old = None;

  cx.execute(|cx| {
    cx.on_update(|w, _| {
      scene_old = w.replace_target_scene(widget_scene).into();
    });

    f(cx);
  });

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut viewer_cx.stage {
    if let Some(scene) = scene_old.take() {
      writer.scene = scene
    }
  }
}
