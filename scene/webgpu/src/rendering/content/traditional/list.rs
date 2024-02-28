use core::cmp::Ordering::Equal;

use crate::*;

// pub type ShaderHash = u64;

// pub struct RenderListNormalizationSystem {
//   // should use stream map
//   list: FastHashMap<SceneCamera, OneManyRelationForker<ShaderHash, AllocIdx<SceneModelImpl>>>,
// }
// impl RenderListNormalizationSystem {
//   pub fn new(scene: &Scene) -> Self {
//     // reactive to all camera in the scene
//     todo!()
//   }
// }

// pub struct RenderListGLESSystem {
//   // should use stream map
//   base: RenderListNormalizationSystem, // we should keep base to lookup the shaderhash one to
// many   // the transparent is auto implicitly separate by shader hash
//   // gles always require cpu side sort(for transparent, and for opaque performance)
//   distances: FastHashMap<SceneCamera, RxCForker<AllocIdx<SceneModelImpl>, f32>>,
// }
// impl RenderListGLESSystem {
//   pub fn new(upstream: &RenderListNormalizationSystem) -> Self {
//     todo!()
//   }
// }

pub struct RenderList {
  // hold this view to make sure the alloc idx is valid during the processing
  sm: StorageReadView<SceneModelImpl>,
  pub(crate) opaque: Vec<(AllocIdx<SceneModelImpl>, f32)>,
  pub(crate) transparent: Vec<(AllocIdx<SceneModelImpl>, f32)>,
}

impl RenderList {
  pub fn from_scene(scene: &Scene) -> Self {
    // if scene.scene.active_camera.is_none() {
    //   return RenderList::with_capacity(0, 0);
    // }

    // let camera_mat = camera.visit(|camera| scene.node_derives.get_world_matrix(&camera.node));
    // let camera_pos = camera_mat.position();
    // let camera_forward = camera_mat.forward().reverse();

    // for m in iter {
    //   let model_pos = scene
    //     .node_derives
    //     .get_world_matrix(&m.read().node)
    //     .position();
    //   let depth = (model_pos - camera_pos).dot(camera_forward);

    //   if blend && m.read().model.should_use_alpha_blend() {
    //     self.transparent.push((m.clone(), depth));
    //   } else {
    //     self.opaque.push((m.clone(), depth));
    //   }
    // }
    todo!()
  }

  pub fn with_capacity(opaque_size: usize, transparent_size: usize) -> Self {
    Self {
      sm: storage_of::<SceneModelImpl>().read(),
      opaque: Vec::with_capacity(opaque_size),
      transparent: Vec::with_capacity(transparent_size),
    }
  }

  pub fn sort(&mut self) {
    // sort front to back
    self
      .opaque
      .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Equal));
    // sort back to front
    self
      .transparent
      .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Equal));
  }
}

pub struct RenderListRender<'a> {
  list: &'a RenderList,
  res: &'a SceneModelGPUResource,
}

impl<'a> SceneRenderable for RenderListRender<'a> {
  fn render(
    &self,
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneRenderCameraCtx,
  ) {
    for (sm_id, _) in &self.list.opaque {
      let sm = self.list.sm.get(*sm_id).unwrap();
      SceneModelRender {
        model: sm,
        res: self.res,
        override_node: None,
      }
      .render(pass, dispatcher, camera);
    }
    for (sm_id, _) in &self.list.transparent {
      let sm = self.list.sm.get(*sm_id).unwrap();
      SceneModelRender {
        model: sm,
        res: self.res,
        override_node: None,
      }
      .render(pass, dispatcher, camera);
    }
  }
}
