use crate::*;

pub struct GlobalGPUSystem {
  pub content: Arc<RwLock<ContentGPUSystem>>,
  pub scenes: StreamMap<usize, SceneGPUSystem>,
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
  ) -> (&mut SceneGPUSystem, &RwLock<ContentGPUSystem>) {
    let scene = self.scenes.get_or_insert_with(scene.guid(), || {
      SceneGPUSystem::new(scene, derives, self.content.clone())
    });

    // the new created scene sys requires maintain
    do_updates(scene, |_| {});
    (scene, &self.content)
  }

  pub fn maintain(&mut self) {
    let mut content = self.content.write().unwrap();
    let content: &mut ContentGPUSystem = &mut content;
    do_updates(content, |_| {});

    do_updates(&mut self.scenes, |_| {});
  }
}
