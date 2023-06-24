use crate::*;

#[pin_project::pin_project]
#[derive(Default)]
pub struct SingleProjectShadowMapSystem {
  /// light guid to light shadow camera
  #[pin]
  cameras: StreamMap<usize, ReactiveBasicShadowSceneCamera>,
  #[pin]
  shadow_maps: StreamMap<usize, ShadowMap>,
  maps: ShadowMapAllocator,
  list: BasicShadowMapInfoList,
  emitter: HashMap<usize, futures::channel::mpsc::UnboundedSender<LightShadowAddressInfo>>,
}

impl SingleProjectShadowMapSystem {
  pub fn create_shadow_info_stream(
    &self,
    light_id: usize,
    proj: impl Stream<Item = (Box<dyn CameraProjection>, Size)>,
    node_delta: impl Stream<Item = SceneNode>,
  ) -> impl Stream<Item = LightShadowAddressInfo> {
    let camera_stream = basic_shadow_camera(proj, node_delta);
    self.cameras.insert(light_id, camera_stream);
    let (sender, rec) = futures::channel::mpsc::unbounded();
    self.emitter.insert(light_id, sender);
    rec
  }

  pub fn maintain(&mut self) {
    do_updates(&mut self.cameras, |updates| match updates {
      StreamMapDelta::Delta(idx, delta) => {
        if let Some((_, size)) = delta {
          self.shadow_maps.remove(idx);
          self.shadow_maps.insert(idx, self.maps.allocate(size));
        } else {
          self.shadow_maps.remove(idx)
        }
      }
      StreamMapDelta::Remove(idx) => {
        self.emitter.remove(&idx);
      }
      _ => {}
    })
  }

  pub fn update_depth_maps(&mut self, ctx: &FrameCtx) {
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
      let (view, map_info) = map.get_write_view(ctx.ctx.gpu);

      pass("shadow-depth")
        .with_depth(view, clear(1.))
        .render(ctx.ctx)
        .by(CameraSceneRef {
          camera: &shadow_camera,
          scene: ctx.scene,
          inner: SceneDepth,
        });
    }
  }
}

type ReactiveBasicShadowSceneCamera = impl Stream<Item = (SceneCamera, Size)>;

fn basic_shadow_camera(
  proj: impl Stream<Item = (Box<dyn CameraProjection>, Size)>,
  node_delta: impl Stream<Item = SceneNode>,
) -> ReactiveBasicShadowSceneCamera {
  proj
    .zip_signal(node_delta)
    .map(|((p, size), node)| (SceneCamera::create(proj, node), size))
}

pub const SHADOW_MAX: usize = 8;

#[derive(Default)]
pub struct BasicShadowMapInfoList {
  pub list: ClampedUniformList<BasicShadowMapInfo, SHADOW_MAX>,
}

only_fragment!(BasicShadowMapInfoGroup, Shader140Array<BasicShadowMapInfo, SHADOW_MAX>);

impl RebuildAbleGPUCollectionBase for BasicShadowMapInfoList {
  fn reset(&mut self) {
    self.list.reset();
  }

  fn update_gpu(&mut self, gpu: &GPU) -> usize {
    self.list.update_gpu(gpu)
  }
}

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
