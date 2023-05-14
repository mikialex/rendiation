use crate::*;

pub const SHADOW_MAX: usize = 8;

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

impl Default for BasicShadowMapInfoList {
  fn default() -> Self {
    Self {
      list: ClampedUniformList::default_with(SB::Pass),
    }
  }
}

impl ShaderGraphProvider for BasicShadowMapInfoList {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let list = binding.uniform_by(self.list.gpu.as_ref().unwrap(), SB::Pass);
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

pub struct BasicShadowGPU {
  pub shadow_camera: SceneCamera,
  pub map: ShadowMap,
  pub uniform: BasicShadowMapInfo,
}

pub trait ShadowSingleProjectCreator {
  fn build_shadow_projection(&self) -> Option<impl Stream<Item = Box<dyn CameraProjection>>>;
}

#[derive(Default)]
struct SingleProjectShadowMapSystem {
  /// light guid to light shadow camera
  cameras: StreamMap<usize, ReactiveBasicShadowSceneCamera>,
  maps: ShadowMapAllocator,
  shadow_maps: StreamMap<usize, ShadowMap>,
  list: BasicShadowMapInfoList,
}

impl SingleProjectShadowMapSystem {
  pub fn create_shadow_info_stream(
    &self,
    light: &SceneLight,
  ) -> impl Stream<Item = LightShadowAddressInfo> {
    let camera_stream = basic_shadow_camera(light);
    self.cameras.insert(light.guid, camera_stream);
    let shadow_map = self.maps.allocate(resolution);
  }

  pub fn update_depth(&mut self, ctx: &FrameCtx) {
    //
  }
}

type ReactiveBasicShadowSceneCamera =
  impl Stream<Item = Option<(SceneCamera, Size)>> + AsRef<Option<(SceneCamera, Size)>>;

fn basic_shadow_camera(light: SceneLight) -> ReactiveBasicShadowSceneCamera {
  let l = light.read();
  let proj = l.light.build_shadow_projection()?;
  let camera = SceneCamera::create_camera_inner(proj, l.node.clone());

  light.single_listen_by(with_field!(SceneLightInner => node));

  light
    .single_listen_by(with_field!(SceneLightInner => light))
    .filter_map_sync(|light: SceneLightKind| light.build_shadow_projection());

  light
    .unbound_listen_by(all_delta)
    .fold_signal(camera, |delta, camera| {
      match delta {
        SceneLightInnerDelta::light(l) => {
          let proj = l.build_shadow_projection().unwrap(); // todo
          SceneCameraInnerDelta::projection(proj).apply_modify(camera)
        }
        SceneLightInnerDelta::node(n) => SceneCameraInnerDelta::node(n).apply_modify(camera),
      }
      Some(())
    })
    .into()
}

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

impl BasicShadowGPU {
  pub fn update_basic_shadow_map(&self, ctx: &mut LightingCtx) -> LightShadowAddressInfo {
    let BasicShadowGPU { shadow_camera, map } = self;

    let (view, map_info) = map.get_write_view(ctx.ctx.gpu);

    // let shadow_camera_info = ctx
    //   .scene
    //   .scene_resources
    //   .cameras
    //   .write()
    //   .unwrap()
    //   .get_or_insert(
    //     &shadow_camera,
    //     ctx.scene.node_derives,
    //     &ctx.scene.resources.gpu,
    //   )
    //   .as_ref()
    //   .inner
    //   .ubo
    //   .resource
    //   .get();

    pass("shadow-depth")
      .with_depth(view, clear(1.))
      .render(ctx.ctx)
      .by(CameraSceneRef {
        camera: &shadow_camera,
        scene: ctx.scene,
        inner: SceneDepth,
      });

    // let shadows = ctx
    //   .shadows
    //   .shadow_collections
    //   .entry(TypeId::of::<BasicShadowMapInfoList>())
    //   .or_insert_with(|| Box::<BasicShadowMapInfoList>::default());

    // let shadows = shadows
    //   .as_any_mut()
    //   .downcast_mut::<BasicShadowMapInfoList>()
    //   .unwrap();

    // let index = shadows.list.source.len();

    // let mut info = BasicShadowMapInfo::default();
    // info.shadow_camera = shadow_camera_info;
    // info.bias = ShadowBias::new(-0.0001, 0.0);
    // info.map_info = map_info;

    // shadows.list.source.push(info);

    // LightShadowAddressInfo::new(true, index as u32)
  }
}
