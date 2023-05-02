use crate::*;

pub struct SceneNodeGPUSystem {
  nodes: SceneNodeGPUStorage,
}

pub type ReactiveNodeGPU =
  impl Stream<Item = RenderComponentDeltaFlag> + AsRef<RenderComponentCell<NodeGPU>> + Unpin;

pub type SceneNodeGPUStorage =
  impl AsRef<StreamVec<ReactiveNodeGPU>> + Stream<Item = VecUpdateUnit<RenderComponentDeltaFlag>>;

impl SceneNodeGPUSystem {
  pub fn new(scene: &Scene, derives: &SceneNodeDeriveSystem, cx: &ResourceGPUCtx) -> Self {
    fn build_reactive_node(mat: WorldMatrixStream, cx: &ResourceGPUCtx) -> ReactiveNodeGPU {
      let node = NodeGPU::new(&cx.device);
      let state = RenderComponentCell::new(node);

      let cx = cx.clone();

      mat.fold_signal(state, move |delta, state| {
        state.inner.update(&cx.queue, delta);
        RenderComponentDeltaFlag::Content.into()
      })
    }

    let derives = derives.clone();
    let cx = cx.clone();

    let nodes = scene
      .unbound_listen_by(|view, send| match view {
        MaybeDeltaRef::All(scene) => scene.nodes.expand(send),
        MaybeDeltaRef::Delta(delta) => {
          if let SceneInnerDelta::nodes(node_d) = delta {
            send(node_d.clone())
          }
        }
      })
      .filter_map_sync(move |v| match v {
        tree::TreeMutation::Create { data, node: idx } => {
          let world_st = derives.create_world_matrix_stream_by_raw_handle(idx);
          let node = build_reactive_node(world_st, &cx);
          (idx, node.into()).into()
        }
        tree::TreeMutation::Delete(idx) => (idx, None).into(),
        _ => None,
      })
      .flatten_into_vec_stream_signal();

    Self { nodes }
  }
}

// pub struct ReactiveNodeGPU {
//   pub ubo: UniformBufferDataView<TransformGPUData>,
// }

// impl ReactiveNodeGPU {
//     pub fn new() -> Self{

//     }
// }
// struct SceneCameraGPUSystem;
// struct SceneBundleGPUSystem;

#[pin_project::pin_project]
pub struct SceneGPUSystem {
  gpu: ResourceGPUCtx,
  pub nodes: SceneNodeGPUSystem,
  // // the camera gpu data are mostly related to scene node it used, so keep it at scene level;
  // cameras: SceneCameraGPUSystem,
  // bundle: SceneBundleGPUSystem,
  #[pin]
  models: Arc<RwLock<StreamMap<ReactiveSceneModelGPUInstance>>>,

  #[pin]
  source: SceneGPUUpdateSource,

  pub cameras: RefCell<CameraGPUMap>,
  pub lights: RefCell<GPULightCache>,
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
    // early_return_ready!(this.nodes.nodes.poll_next(cx));

    let mut models = this.models.write().unwrap();
    let models: &mut StreamMap<ReactiveSceneModelGPUInstance> = &mut models;
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
    let models: Arc<RwLock<StreamMap<ReactiveSceneModelGPUInstance>>> = Default::default();
    let models_c = models.clone();
    let gpu = contents.read().unwrap().gpu.clone();

    let nodes = SceneNodeGPUSystem::new(scene, derives, &gpu);

    let source = scene.unbound_listen_by(all_delta).map(move |delta| {
      let contents = contents.write().unwrap();
      let mut models = models_c.write().unwrap();
      let models: &mut StreamMap<ReactiveSceneModelGPUInstance> = &mut models;
      if let SceneInnerDelta::models(delta) = delta {
        match delta {
          arena::ArenaDelta::Mutate((model, _)) => {
            models.remove(model.id());
            models.get_or_insert_with(model.id(), || build_scene_model_gpu(&model, &contents));
          }
          arena::ArenaDelta::Insert((model, _)) => {
            models.get_or_insert_with(model.id(), || build_scene_model_gpu(&model, &contents));
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
      // nodes: (),
      // cameras: (),
      // bundle: (),
      source,
      cameras: Default::default(),
      nodes,
      lights: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    do_updates(self, |_| {});
  }
}
