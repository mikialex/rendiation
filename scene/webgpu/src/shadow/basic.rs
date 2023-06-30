use crate::*;

#[pin_project::pin_project]
pub struct SingleProjectShadowMapSystem {
  /// light guid to light shadow camera
  #[pin]
  cameras: StreamMap<usize, ReactiveBasicShadowSceneCamera>,
  #[pin]
  shadow_maps: StreamMap<usize, ShadowMap>,
  maps: ShadowMapAllocator,
  pub list: BasicShadowMapInfoList,
  gpu: ResourceGPUCtx,
  emitter: HashMap<usize, futures::channel::mpsc::UnboundedSender<LightShadowAddressInfo>>,
}

impl SingleProjectShadowMapSystem {
  pub fn new(gpu: ResourceGPUCtx, maps: ShadowMapAllocator) -> Self {
    Self {
      cameras: Default::default(),
      shadow_maps: Default::default(),
      maps,
      list: Default::default(),
      gpu,
      emitter: Default::default(),
    }
  }

  pub fn create_shadow_info_stream(
    &mut self,
    light_id: usize,
    proj: impl Stream<Item = (CameraProjector, Size)> + Unpin,
    node_delta: impl Stream<Item = SceneNode> + Unpin,
  ) -> impl Stream<Item = LightShadowAddressInfo> {
    let camera_stream = basic_shadow_camera(proj, node_delta);
    self.cameras.insert(light_id, camera_stream);
    let (sender, rec) = futures::channel::mpsc::unbounded();
    // list is not support reordering yet, should emit initial value now
    sender.unbounded_send(LightShadowAddressInfo::new(
      true,
      self.list.allocate() as u32,
    ));
    self.emitter.insert(light_id, sender);
    rec
  }

  pub fn maintain(&mut self) {
    do_updates(&mut self.cameras, |updates| match updates {
      StreamMapDelta::Delta(idx, (_camera, size)) => {
        self.shadow_maps.remove(idx);
        self.shadow_maps.insert(idx, self.maps.allocate(size));

        let index = self.list.mapping.get(&idx).unwrap();
        self.list.list.source[*index].shadow_camera = todo!();
      }
      StreamMapDelta::Remove(idx) => {
        self.emitter.remove(&idx);
      }
      _ => {}
    });

    do_updates(&mut self.shadow_maps, |updates| match updates {
      StreamMapDelta::Delta(idx, delta) => {
        let index = self.list.mapping.get(&idx).unwrap();
        self.list.list.source[*index].map_info = delta;
      }
      _ => {}
    });

    self.list.list.update_gpu(&self.gpu.device);
  }

  pub fn update_depth_maps(&mut self, ctx: &mut FrameCtx, scene: &SceneRenderResourceGroup) {
    assert_eq!(self.cameras.streams.len(), self.shadow_maps.streams.len());

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

    for (light_id, shadow_camera) in &self.cameras.streams {
      let map = self.shadow_maps.streams.get(light_id).unwrap();
      let (view, map_info) = map.get_write_view(ctx.gpu);

      pass("shadow-depth")
        .with_depth(view, clear(1.))
        .render(ctx)
        .by(CameraSceneRef {
          camera: shadow_camera,
          scene,
          inner: SceneDepth,
        });
    }
  }
}

type ReactiveBasicShadowSceneCamera = impl Stream<Item = (SceneCamera, Size)> + Unpin;

fn basic_shadow_camera(
  proj: impl Stream<Item = (CameraProjector, Size)> + Unpin,
  node_delta: impl Stream<Item = SceneNode> + Unpin,
) -> ReactiveBasicShadowSceneCamera {
  proj
    .zip(node_delta)
    .map(|((p, size), node)| (SceneCamera::create(p, node), size))
}

const SHADOW_MAX: usize = 8;

struct BasicShadowMapInfoList {
  list: ClampedUniformList<BasicShadowMapInfo, SHADOW_MAX>,
  /// map light id to index;
  empty_list: Vec<usize>,
  mapping: HashMap<usize, usize>,
}

impl BasicShadowMapInfoList {
  // todo, return stream
  pub fn allocate(&mut self) -> usize {
    todo!()
  }
}

impl Default for BasicShadowMapInfoList {
  fn default() -> Self {
    Self {
      list: Default::default(),
      mapping: Default::default(),
      empty_list: (0..SHADOW_MAX).into_iter().collect(),
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
