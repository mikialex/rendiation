use crate::*;

#[pin_project::pin_project]
pub struct SceneGPUSystem {
  pub gpu: ResourceGPUCtx,

  #[pin]
  pub nodes: SceneNodeGPUSystem,

  #[pin]
  pub cameras: SceneCameraGPUSystem,

  #[pin]
  pub models: Arc<RwLock<StreamMap<usize, ReactiveSceneModelGPUInstance>>>,

  #[pin]
  source: SceneGPUUpdateSource,

  pub lights: RefCell<GPULightCache>,
}

#[derive(Default)]
pub struct GPULightCache {
  pub inner: HashMap<TypeId, Box<dyn Any>>,
}

impl SceneGPUSystem {
  pub fn encode(&self, _encoder: &mut GPUCommandEncoder, _pass_dispatcher: &dyn RenderComponent) {
    // do encoding
  }
}

impl Stream for SceneGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    early_return_ready!(this.source.poll_next(cx));
    early_return_ready!(this.nodes.poll_next(cx));
    early_return_ready!(this.cameras.poll_next(cx));

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<usize, ReactiveSceneModelGPUInstance> = &mut models;
    do_updates_by(models, cx, |_| {});
    Poll::Pending
  }
}
type SceneGPUUpdateSource = impl Stream<Item = ()> + Unpin;

impl SceneGPUSystem {
  pub fn new(
    scene: &Scene,
    derives: &SceneNodeDeriveSystem,
    contents: Arc<RwLock<ContentGPUSystem>>,
  ) -> Self {
    let models: Arc<RwLock<StreamMap<usize, ReactiveSceneModelGPUInstance>>> = Default::default();
    let models_c = models.clone();
    let gpu = contents.read().unwrap().gpu.clone();

    let nodes = SceneNodeGPUSystem::new(scene, derives, &gpu);
    let cameras = SceneCameraGPUSystem::new(scene, derives, &gpu);

    let source = scene.unbound_listen_by(all_delta).map(move |delta| {
      let contents = contents.write().unwrap();
      let mut models = models_c.write().unwrap();
      let models: &mut StreamMap<usize, ReactiveSceneModelGPUInstance> = &mut models;
      if let SceneInnerDelta::models(delta) = delta {
        match delta {
          arena::ArenaDelta::Mutate((model, _)) => {
            models.remove(model.guid());
            models.get_or_insert_with(model.guid(), || build_scene_model_gpu(&model, &contents));
          }
          arena::ArenaDelta::Insert((model, _)) => {
            models.get_or_insert_with(model.guid(), || build_scene_model_gpu(&model, &contents));
          }
          arena::ArenaDelta::Remove(handle) => {
            models.remove(handle.index());
          }
        }
      }
    });

    Self {
      gpu,
      models,
      source,
      cameras,
      nodes,
      lights: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    do_updates(self, |_| {});
  }
}
