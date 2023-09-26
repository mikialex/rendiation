use crate::*;

#[pin_project::pin_project]
pub struct SceneGPUSystem {
  pub gpu: ResourceGPUCtx,

  #[pin]
  source: SceneGPUUpdateSource,

  #[pin]
  pub nodes: SceneNodeGPUSystem,

  #[pin]
  pub cameras: RwLock<SceneCameraGPUSystem>, // todo remove lock?

  #[pin]
  pub models: Arc<RwLock<StreamMap<u64, ReactiveSceneModelGPUInstance>>>,

  pub shadows: ShadowMapSystem,
  pub lights: ForwardLightingSystem,
}

impl Stream for SceneGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    if this
      .source
      .poll_until_pending_or_terminate_not_care_result(cx)
    {
      return Poll::Ready(None);
    }
    this.nodes.poll_until_pending_not_care_result(cx);

    let mut cameras_ = this.cameras.write().unwrap();
    let cameras: &mut SceneCameraGPUSystem = &mut cameras_;
    cameras.poll_until_pending_not_care_result(cx);
    drop(cameras_);

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<u64, ReactiveSceneModelGPUInstance> = &mut models;
    models.poll_until_pending_not_care_result(cx);

    this.lights.poll_until_pending_not_care_result(cx);

    let mut cameras = this.cameras.write().unwrap();
    let cameras: &mut SceneCameraGPUSystem = &mut cameras;
    this.shadows.maintain(cameras, cx);

    Poll::Pending
  }
}
impl FusedStream for SceneGPUSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}
type SceneGPUUpdateSource = impl Stream<Item = ()> + Unpin;

impl SceneGPUSystem {
  pub fn new(
    scene: &Scene,
    derives: &SceneNodeDeriveSystem,
    contents: Arc<RwLock<ContentGPUSystem>>,
  ) -> Self {
    let models: Arc<RwLock<StreamMap<u64, ReactiveSceneModelGPUInstance>>> = Default::default();
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
      let models: &mut StreamMap<u64, ReactiveSceneModelGPUInstance> = &mut models;
      if let SceneInternalDelta::models(delta) = delta {
        match delta {
          arena::ArenaDelta::Mutate((model, _)) => {
            models.remove(model.guid());
            models.get_or_insert_with(model.guid(), || build_scene_model_gpu(&model, &contents));
          }
          arena::ArenaDelta::Insert((model, _)) => {
            models.get_or_insert_with(model.guid(), || build_scene_model_gpu(&model, &contents));
          }
          arena::ArenaDelta::Remove(_handle) => {
            // models.remove(handle.index()); todo fixme
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
}
