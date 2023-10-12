use crate::*;

#[pin_project::pin_project]
pub struct SingleProjectShadowMapSystem {
  #[pin]
  cameras_source: StreamMap<u64, ReactiveBasicShadowSceneCamera>,
  cameras: FastHashMap<u64, SceneCamera>,
  cameras_id_map_light_id: FastHashMap<u64, u64>,
  #[pin]
  shadow_maps: StreamMap<u64, ShadowMap>,
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
      cameras_id_map_light_id: Default::default(),
      maps,
      list: Default::default(),
      gpu,
      derives,
    }
  }

  pub fn create_shadow_info_stream(
    &mut self,
    light_id: u64,
    proj: impl Stream<Item = (CameraProjectionEnum, Size)> + Unpin + 'static,
    node_delta: impl Stream<Item = SceneNode> + Unpin + 'static,
  ) -> impl Stream<Item = LightShadowAddressInfo> {
    let camera_stream = basic_shadow_camera(Box::new(proj), Box::new(node_delta));
    self.cameras_source.insert(light_id, camera_stream);
    self.list.allocate(light_id)
  }

  pub fn poll_updates(&mut self, gpu_cameras: &mut SceneCameraGPUSystem, cx: &mut Context) {
    self.cameras_source.poll_until_pending(cx, |updates| {
      for update in updates {
        match update {
          StreamMapDelta::Delta(idx, (camera, size)) => {
            self.shadow_maps.remove(idx); // deallocate map
            self.shadow_maps.insert(idx, self.maps.allocate(size));
            // create the gpu camera
            gpu_cameras.get_or_insert(&camera, &self.derives, &self.gpu);
            self.cameras_id_map_light_id.insert(camera.guid(), idx);
            self.cameras.insert(idx, camera);
          }
          StreamMapDelta::Remove(idx) => {
            self.list.deallocate(idx);
            let camera = self.cameras.remove(&idx).unwrap();
            self.cameras_id_map_light_id.remove(&camera.guid());
          }
          _ => {}
        }
      }
    });

    gpu_cameras.poll_until_pending(cx, |updates| {
      for update in updates {
        if let StreamMapDelta::Delta(camera_id, shadow_camera) = update {
          let light_id = self.cameras_id_map_light_id.get(&camera_id).unwrap();
          self.list.get_mut_data(*light_id).shadow_camera = shadow_camera;
        }
      }
    });

    self.shadow_maps.poll_until_pending(cx, |updates| {
      for update in updates {
        if let StreamMapDelta::Delta(idx, delta) = update {
          self.list.get_mut_data(idx).map_info = delta;
        }
      }
    });

    self.list.list.update_gpu(&self.gpu.device);
  }

  pub fn update_depth_maps(&mut self, ctx: &mut FrameCtx, scene: &SceneRenderResourceGroup) {
    assert_eq!(self.cameras_source.len(), self.shadow_maps.len());

    struct SceneDepth;

    impl PassContentWithSceneAndCamera for SceneDepth {
      fn render(
        &mut self,
        pass: &mut FrameRenderPass,
        scene: &SceneRenderResourceGroup,
        camera: &SceneCamera,
      ) {
        let mut render_list = RenderList::default();
        render_list.collect_from_scene(scene, camera, false);
        let render_list = MaybeBindlessMeshRenderList::from_list(render_list, scene);

        // we could just use default, because the color channel not exist at all
        let base = default_dispatcher(pass);
        let base = &scene.extend_bindless_resource_provider(&base) as &dyn RenderComponentAny;

        render_list.setup_pass(pass, &base, camera, scene);
      }
    }

    for (light_id, camera) in &self.cameras {
      let map = self.shadow_maps.get(light_id).unwrap();
      let (view, _map_info) = map.get_write_view();

      pass("shadow-depth")
        .with_depth(view, clear(1.))
        .render_ctx(ctx)
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
  proj: Box<dyn Stream<Item = (CameraProjectionEnum, Size)> + Unpin>,
  node_delta: Box<dyn Stream<Item = SceneNode> + Unpin>,
) -> ReactiveBasicShadowSceneCamera {
  proj
    .zip(node_delta)
    .map(|((p, size), node)| (SceneCameraImpl::new(p, node).into_ptr(), size))
}

const SHADOW_MAX: usize = 8;

pub struct BasicShadowMapInfoList {
  list: ClampedUniformList<BasicShadowMapInfo, SHADOW_MAX>,
  /// map light id to index;
  empty_list: Vec<usize>,
  waker: futures::task::AtomicWaker,
  mapping: FastHashMap<u64, usize>,
  // todo support reordering?
  emitter: FastHashMap<u64, futures::channel::mpsc::UnboundedSender<LightShadowAddressInfo>>,
}

impl Stream for BasicShadowMapInfoList {
  // emit new real size
  type Item = usize;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    self.waker.register(cx.waker());
    todo!() // and gpu update and reorder
  }
}

impl BasicShadowMapInfoList {
  fn allocate(&mut self, light_id: u64) -> impl Stream<Item = LightShadowAddressInfo> {
    self.waker.wake();
    let idx = self.empty_list.pop().unwrap();
    let (sender, rec) = futures::channel::mpsc::unbounded();
    sender
      .unbounded_send(LightShadowAddressInfo::new(true, idx as u32))
      .ok();
    self.emitter.insert(light_id, sender);
    self.mapping.insert(light_id, idx);
    rec
  }
  fn deallocate(&mut self, light_id: u64) {
    self.waker.wake();
    let index = self.mapping.remove(&light_id).unwrap();
    self.empty_list.push(index);
    self.emitter.remove(&light_id);
  }
  fn get_mut_data(&mut self, light_id: u64) -> &mut BasicShadowMapInfo {
    self.waker.wake();
    let index = self.mapping.get(&light_id).unwrap();
    while self.list.source.len() <= *index {
      self.list.source.push(Default::default());
    }
    &mut self.list.source[*index]
  }
}

impl Default for BasicShadowMapInfoList {
  fn default() -> Self {
    Self {
      list: Default::default(),
      mapping: Default::default(),
      empty_list: (0..SHADOW_MAX).rev().collect(),
      emitter: Default::default(),
      waker: Default::default(),
    }
  }
}

only_fragment!(
  BasicShadowMapInfoGroup,
  ShaderUniformPtr<Shader140Array<BasicShadowMapInfo, SHADOW_MAX>>
);

impl GraphicsShaderProvider for BasicShadowMapInfoList {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.bind_by(self.list.gpu.as_ref().unwrap());
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
