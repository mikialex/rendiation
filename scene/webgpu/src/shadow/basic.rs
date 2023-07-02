use crate::*;

#[pin_project::pin_project]
pub struct SingleProjectShadowMapSystem {
  #[pin]
  cameras_source: StreamMap<usize, ReactiveBasicShadowSceneCamera>,
  cameras: HashMap<usize, SceneCamera>,
  #[pin]
  shadow_maps: StreamMap<usize, ShadowMap>,
  maps: ShadowMapAllocator,
  pub list: BasicShadowMapInfoList,
  gpu: ResourceGPUCtx,
  derives: SceneNodeDeriveSystem,
}

impl SingleProjectShadowMapSystem {
  pub fn new(
    gpu: ResourceGPUCtx,
    maps: ShadowMapAllocator,
    derives: SceneNodeDeriveSystem,
  ) -> Self {
    Self {
      cameras_source: Default::default(),
      shadow_maps: Default::default(),
      cameras: Default::default(),
      maps,
      list: Default::default(),
      gpu,
      derives,
    }
  }

  pub fn create_shadow_info_stream(
    &mut self,
    light_id: usize,
    proj: impl Stream<Item = (CameraProjector, Size)> + Unpin + 'static,
    node_delta: impl Stream<Item = SceneNode> + Unpin + 'static,
  ) -> impl Stream<Item = LightShadowAddressInfo> {
    let camera_stream = basic_shadow_camera(Box::new(proj), Box::new(node_delta));
    self.cameras_source.insert(light_id, camera_stream);
    self.list.allocate(light_id)
  }

  pub fn maintain(&mut self, gpu_cameras: &mut SceneCameraGPUSystem, cx: &mut Context) {
    do_updates_by(&mut self.cameras_source, cx, |updates| match updates {
      StreamMapDelta::Delta(idx, (camera, size)) => {
        self.shadow_maps.remove(idx); // deallocate map
        self.shadow_maps.insert(idx, self.maps.allocate(size));
        // create the gpu camera
        gpu_cameras.get_or_insert(&camera, &self.derives, &self.gpu);
        self.cameras.insert(idx, camera);
      }
      StreamMapDelta::Remove(idx) => {
        self.list.deallocate(idx);
        self.cameras.remove(&idx);
      }
      _ => {}
    });

    do_updates(gpu_cameras, |d| {
      if let StreamMapDelta::Delta(idx, shadow_camera) = d {
        self.list.get_mut_data(idx).shadow_camera = shadow_camera;
      }
    });

    do_updates(&mut self.shadow_maps, |updates| {
      if let StreamMapDelta::Delta(idx, delta) = updates {
        self.list.get_mut_data(idx).map_info = delta;
      }
    });

    self.list.list.update_gpu(&self.gpu.device);
  }

  pub fn update_depth_maps(&mut self, ctx: &mut FrameCtx, scene: &SceneRenderResourceGroup) {
    assert_eq!(
      self.cameras_source.streams.len(),
      self.shadow_maps.streams.len()
    );

    struct SceneDepth;

    impl PassContentWithSceneAndCamera for SceneDepth {
      fn render(
        &mut self,
        pass: &mut FrameRenderPass,
        scene: &SceneRenderResourceGroup,
        camera: &SceneCamera,
      ) {
        let mut render_list = RenderList::default();
        render_list.prepare(scene, camera);

        // we could just use default, because the color channel not exist at all
        let base = default_dispatcher(pass);

        render_list.setup_pass(pass, &base, camera, scene);
      }
    }

    for (light_id, camera) in &self.cameras {
      let map = self.shadow_maps.streams.get(light_id).unwrap();
      let (view, _map_info) = map.get_write_view();

      pass("shadow-depth")
        .with_depth(view, clear(1.))
        .render(ctx)
        .by(CameraSceneRef {
          camera,
          scene,
          inner: SceneDepth,
        });
    }
  }
}

type ReactiveBasicShadowSceneCamera = impl Stream<Item = (SceneCamera, Size)> + Unpin;

// todo remove box
fn basic_shadow_camera(
  proj: Box<dyn Stream<Item = (CameraProjector, Size)> + Unpin>,
  node_delta: Box<dyn Stream<Item = SceneNode> + Unpin>,
) -> ReactiveBasicShadowSceneCamera {
  proj
    .zip(node_delta)
    .map(|((p, size), node)| (SceneCamera::create(p, node), size))
}

const SHADOW_MAX: usize = 8;

pub struct BasicShadowMapInfoList {
  list: ClampedUniformList<BasicShadowMapInfo, SHADOW_MAX>,
  /// map light id to index;
  empty_list: Vec<usize>,
  mapping: HashMap<usize, usize>,
  // todo support reordering?
  emitter: HashMap<usize, futures::channel::mpsc::UnboundedSender<LightShadowAddressInfo>>,
}

impl BasicShadowMapInfoList {
  fn allocate(&mut self, light_id: usize) -> impl Stream<Item = LightShadowAddressInfo> {
    let idx = self.empty_list.pop().unwrap();
    let (sender, rec) = futures::channel::mpsc::unbounded();
    sender
      .unbounded_send(LightShadowAddressInfo::new(true, idx as u32))
      .ok();
    self.emitter.insert(light_id, sender);
    self.mapping.insert(light_id, idx);
    rec
  }
  fn deallocate(&mut self, light_id: usize) {
    let index = self.mapping.remove(&light_id).unwrap();
    self.empty_list.push(index);
    self.emitter.remove(&light_id);
  }
  fn get_mut_data(&mut self, light_id: usize) -> &mut BasicShadowMapInfo {
    let index = self.mapping.get(&light_id).unwrap();
    &mut self.list.source[*index]
  }
}

impl Default for BasicShadowMapInfoList {
  fn default() -> Self {
    Self {
      list: Default::default(),
      mapping: Default::default(),
      empty_list: (0..SHADOW_MAX).collect(),
      emitter: Default::default(),
    }
  }
}

only_fragment!(BasicShadowMapInfoGroup, Shader140Array<BasicShadowMapInfo, SHADOW_MAX>);

impl ShaderGraphProvider for BasicShadowMapInfoList {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.uniform_by(self.list.gpu.as_ref().unwrap());
      builder.register::<BasicShadowMapInfoGroup>(list);
      Ok(())
    })
  }
}
impl ShaderHashProvider for BasicShadowMapInfoList {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.list.hash_pipeline(hasher)
  }
}
impl ShaderPassBuilder for BasicShadowMapInfoList {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.list.setup_pass(ctx)
  }
}
