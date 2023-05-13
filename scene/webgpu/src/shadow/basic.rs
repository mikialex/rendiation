use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LightShadowAddressInfo {
  pub index: u32,
  pub enabled: u32,
}

impl LightShadowAddressInfo {
  pub fn new(enabled: bool, index: u32) -> Self {
    Self {
      enabled: enabled.into(),
      index,
      ..Zeroable::zeroed()
    }
  }
}

only_fragment!(BasicShadowMapInfoGroup, Shader140Array<BasicShadowMapInfo, SHADOW_MAX>);
only_fragment!(BasicShadowMap, ShaderDepthTexture2DArray);
only_fragment!(BasicShadowMapSampler, ShaderCompareSampler);

pub fn compute_shadow_position(
  builder: &ShaderGraphFragmentBuilderView,
  shadow_info: ENode<BasicShadowMapInfo>,
) -> Result<Node<Vec3<f32>>, ShaderGraphBuildError> {
  // another way to compute this is in vertex shader, maybe we will try it later.
  let bias = shadow_info.bias.expand();
  let world_position = builder.query::<FragmentWorldPosition>()?;
  let world_normal = builder.query::<FragmentWorldNormal>()?;

  // apply normal bias
  let world_position = world_position + bias.normal_bias * world_normal;

  let shadow_position =
    shadow_info.shadow_camera.expand().view_projection * (world_position, 1.).into();

  let shadow_position = shadow_position.xyz() / shadow_position.w();

  // convert to uv space and apply offset bias
  Ok(
    shadow_position * consts(Vec3::new(0.5, -0.5, 1.))
      + consts(Vec3::new(0.5, 0.5, 0.))
      + (0., 0., bias.bias).into(),
  )
}

pub struct BasicShadowGPU {
  pub shadow_camera: ReactiveBasicShadowSceneCamera,
  pub map: ShadowMap,
}

pub trait ShadowSingleProjectCreator {
  fn build_shadow_projection(&self) -> Option<impl Stream<Item = Box<dyn CameraProjection>>>;
}

struct SingleProjectShadowMapSystem {
  cameras: StreamMap<usize, ReactiveBasicShadowSceneCamera>,
}

impl SingleProjectShadowMapSystem {
  pub fn create_shadow_info_stream(&self) -> impl Stream<Item = LightShadowAddressInfo> {
    //
  }
}

impl SingleProjectShadowMapSystem {
  pub fn new(scene: &Scene) -> Self {
    //
  }
}

type ReactiveBasicShadowSceneCamera = impl Stream<Item = ()> + AsRef<SceneCamera>;

// fn basic_shadow_gpu(light: SceneLight) -> Option<ReactiveBasicShadowSceneCamera> {
//   let l = light.read();
//   let proj = l.light.build_shadow_projection()?;
//   let camera = SceneCamera::create_camera_inner(proj, l.node.clone());

//   light.single_listen_by(with_field!(SceneLightInner => node));

//   light
//     .single_listen_by(with_field!(SceneLightInner => light))
//     .filter_map_sync(|light: SceneLightKind| light.build_shadow_projection());

//   light
//     .unbound_listen_by(all_delta)
//     .fold_signal(camera, |delta, camera| {
//       match delta {
//         SceneLightInnerDelta::light(l) => {
//           let proj = l.build_shadow_projection().unwrap(); // todo
//           SceneCameraInnerDelta::projection(proj).apply_modify(camera)
//         }
//         SceneLightInnerDelta::node(n) => SceneCameraInnerDelta::node(n).apply_modify(camera),
//       }
//       Some(())
//     })
//     .into()
// }

// fn get_shadow_map<T>(
//   inner: &SceneItemRef<T>,
//   resources: &SceneGPUSystem,
//   shadows: &mut ShadowMapSystem,
//   node: &SceneNode,
// ) -> BasicShadowGPU
// where
//   T: Any + Incremental,
//   SceneItemRef<T>: ShadowSingleProjectCreator,
// {
//   let resolution = Size::from_usize_pair_min_one((512, 512));

//   let mut lights = resources.lights.borrow_mut();
//   lights
//     .inner
//     .entry(TypeId::of::<T>())
//     .or_insert_with(|| Box::<IdentityMapper<BasicShadowGPU, T>>::default())
//     .downcast_mut::<IdentityMapper<BasicShadowGPU, T>>()
//     .unwrap()
//     .get_update_or_insert_with_logic(inner, |logic| match logic {
//       ResourceLogic::Create(light) => {
//         let shadow_camera = light.build_shadow_camera(node);
//         let map = shadows.maps.allocate(resolution);
//         ResourceLogicResult::Create(BasicShadowGPU { shadow_camera, map })
//       }
//       ResourceLogic::Update(shadow, light) => {
//         let shadow_camera = light.build_shadow_camera(node);
//         let map = shadows.maps.allocate(resolution);
//         *shadow = BasicShadowGPU { shadow_camera, map };
//         ResourceLogicResult::Update(shadow)
//       }
//     })
//     .clone()
// }

// pub fn request_basic_shadow_map<T>(
//   inner: &SceneItemRef<T>,
//   resources: &SceneGPUSystem,
//   shadows: &mut ShadowMapSystem,
//   node: &SceneNode,
// ) where
//   T: Any + Incremental,
//   SceneItemRef<T>: ShadowSingleProjectCreator,
// {
//   get_shadow_map(inner, resources, shadows, node);
// }

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

pub fn check_update_basic_shadow_map<T>(
  inner: &SceneItemRef<T>,
  ctx: &mut LightingCtx,
  node: &SceneNode,
) -> LightShadowAddressInfo
where
  T: Any + Incremental,
  SceneItemRef<T>: ShadowSingleProjectCreator,
{
  let BasicShadowGPU { shadow_camera, map } =
    get_shadow_map(inner, ctx.scene.scene_resources, ctx.shadows, node);

  let (view, map_info) = map.get_write_view(ctx.ctx.gpu);

  let shadow_camera_info = ctx
    .scene
    .scene_resources
    .cameras
    .write()
    .unwrap()
    .get_or_insert(
      &shadow_camera,
      ctx.scene.node_derives,
      &ctx.scene.resources.gpu,
    )
    .as_ref()
    .inner
    .ubo
    .resource
    .get();

  pass("shadow-depth")
    .with_depth(view, clear(1.))
    .render(ctx.ctx)
    .by(CameraSceneRef {
      camera: &shadow_camera,
      scene: ctx.scene,
      inner: SceneDepth,
    });

  let shadows = ctx
    .shadows
    .shadow_collections
    .entry(TypeId::of::<BasicShadowMapInfoList>())
    .or_insert_with(|| Box::<BasicShadowMapInfoList>::default());

  let shadows = shadows
    .as_any_mut()
    .downcast_mut::<BasicShadowMapInfoList>()
    .unwrap();

  let index = shadows.list.source.len();

  let mut info = BasicShadowMapInfo::default();
  info.shadow_camera = shadow_camera_info;
  info.bias = ShadowBias::new(-0.0001, 0.0);
  info.map_info = map_info;

  shadows.list.source.push(info);

  LightShadowAddressInfo::new(true, index as u32)
}
