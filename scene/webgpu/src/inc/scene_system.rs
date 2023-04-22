use crate::*;

// struct SceneNodeGPUSystem;
// struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

#[pin_project::pin_project]
pub struct SceneGPUSystem {
  // we share it between different scene system(it's global)
  contents: Arc<RwLock<GlobalGPUSystem>>,
  // nodes: SceneNodeGPUSystem,
  // // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  // cameras: SceneCameraGPUSystem,
  // bundle: SceneBundleGPUSystem,
  #[pin]
  models: StreamMap<SceneModelGPUReactive>,

  #[pin]
  source: SceneGPUUpdateSource,
}

impl SceneGPUSystem {
  pub fn render(&self, _encoder: &mut GPUCommandEncoder, _pass_dispatcher: &dyn RenderComponent) {
    // do encoding
  }
}

impl Stream for SceneGPUSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    early_return_ready!(this.source.poll_next(cx));
    this.models.poll_next(cx).map(|v| v.map(|_| {}))
  }
}
type SceneGPUUpdateSource = impl Stream<Item = ()> + Unpin;

impl SceneGPUSystem {
  pub fn new(scene: &Scene, contents: Arc<RwLock<GlobalGPUSystem>>) -> Self {
    let models = Default::default();
    let contents_c = contents.clone();

    let source = scene.listen_by(all_delta).map(move |delta| {
      let contents = contents_c.write().unwrap();
      let mut models = contents.models.write().unwrap();
      if let SceneInnerDelta::models(delta) = delta {
        match delta {
          arena::ArenaDelta::Mutate((model, _)) => {
            models.remove(model.id());
            models.get_or_insert_with(model.id(), || {
              //
              todo!()
            });
          }
          arena::ArenaDelta::Insert((model, _)) => {
            models.get_or_insert_with(model.id(), || {
              //
              todo!()
            });
          }
          arena::ArenaDelta::Remove(handle) => {
            models.remove(handle.index());
          }
        }
      }
    });

    Self {
      contents,
      models,
      // nodes: (),
      // cameras: (),
      // bundle: (),
      source,
    }
  }

  pub fn maintain(&mut self) {
    do_updates(self, |_| {});
  }
}
