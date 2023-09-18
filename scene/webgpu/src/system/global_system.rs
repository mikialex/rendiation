use crate::*;

pub struct GlobalGPUSystem {
  pub content: Arc<RwLock<ContentGPUSystem>>,
  pub scenes: StreamMap<u64, SceneGPUSystem>,
}

impl Stream for GlobalGPUSystem {
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.scenes.poll_until_pending_not_care_result(cx);

    self
      .content
      .write()
      .unwrap()
      .poll_until_pending_not_care_result(cx);

    Poll::Pending
  }
}
impl FusedStream for GlobalGPUSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}

impl GlobalGPUSystem {
  pub fn new(gpu: &GPU, config: BindableResourceConfig) -> Self {
    let content = ContentGPUSystem::new(gpu, config);
    Self {
      content: Arc::new(RwLock::new(content)),
      scenes: Default::default(),
    }
  }

  pub fn get_or_create_scene_sys_with_content(
    &mut self,
    scene: &Scene,
    derives: &SceneNodeDeriveSystem,
    cx: &mut Context,
  ) -> (&mut SceneGPUSystem, &RwLock<ContentGPUSystem>) {
    let scene = self.scenes.get_or_insert_with(scene.guid(), || {
      SceneGPUSystem::new(scene, derives, self.content.clone())
    });

    // the new created scene sys requires maintain
    scene.poll_until_pending_not_care_result(cx);
    (scene, &self.content)
  }
}
