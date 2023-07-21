use crate::*;

#[pin_project::pin_project]
pub struct SceneGPUSystem {
  pub gpu: ResourceGPUCtx,

  #[pin]
  pub nodes: SceneNodeGPUSystem,

  #[pin]
  pub cameras: RwLock<SceneCameraGPUSystem>, // todo remove lock?

  #[pin]
  pub models: Arc<RwLock<StreamMap<usize, ReactiveSceneModelGPUInstance>>>,

  #[pin]
  source: SceneGPUUpdateSource,

  pub shadows: ShadowMapSystem,
  pub lights: ForwardLightingSystem,
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

    let mut cameras = this.cameras.write().unwrap();
    let cameras: &mut SceneCameraGPUSystem = &mut cameras;
    let cameras_poll = cameras.poll_next_unpin(cx).map(|v| v.map(|_| ()));
    early_return_ready!(cameras_poll);

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<usize, ReactiveSceneModelGPUInstance> = &mut models;
    do_updates_by(models, cx, |_| {});

    let mut cameras = this.cameras.write().unwrap();
    let cameras: &mut SceneCameraGPUSystem = &mut cameras;
    this.shadows.maintain(cameras, cx);
    do_updates_by(this.lights, cx, |_| {});
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

    let shadows = ShadowMapSystem::new(gpu.clone(), derives.clone());
    let ctx = LightResourceCtx {
      shadow_system: shadows.single_proj_sys.clone(),
      derives: derives.clone(),
    };

    let lights = ForwardLightingSystem::new(scene, gpu.clone(), ctx);

    let scene = scene.read();
    let scene_core = &scene.core;

    let nodes = SceneNodeGPUSystem::new(scene_core, derives, &gpu);
    let cameras = RwLock::new(SceneCameraGPUSystem::new(scene_core, derives, &gpu));

    let source = scene_core.unbound_listen_by(all_delta).map(move |delta| {
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
      lights,
      shadows,
    }
  }

  pub fn maintain(&mut self) {
    do_updates(self, |_| {});
  }
}
