use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  pub intensity: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub shadow: LightShadowAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct LightShadowAddressInfo {
  pub index: u32,
  pub enabled: Bool,
}

only_fragment!(BasicShadowMapInfoGroup, Shader140Array<BasicShadowMapInfo, SHADOW_MAX>);
only_fragment!(BasicShadowMap, ShaderDepthTexture2DArray);
only_fragment!(BasicShadowMapSampler, ShaderCompareSampler);

wgsl_fn!(
  fn directional_shadow_occlusion(
    shadow_position: vec3<f32>,
    map: texture_depth_2d_array,
    d_sampler: comparison_sampler,
    info: ShadowMapAddressInfo,
  ) -> f32 {

    // maybe we could use sampler's border color config, but that's not part of standard webgpu (wgpu supports)
    let inFrustumVec = vec4<bool>(shadow_position.x >= 0.0, shadow_position.x <= 1.0, shadow_position.y >= 0.0, shadow_position.y <= 1.0);
    let inFrustum = all(inFrustumVec);
    let frustumTestVec = vec2<bool>(inFrustum, shadow_position.z <= 1.0);
    let frustumTest = all(frustumTestVec);

    if (frustumTest) {
      return textureSampleCompareLevel(
        map,
        d_sampler,
        shadow_position.xy,
        info.layer_index,
        shadow_position.z,
      );
    } else {
      return 1.0;
    }
  }
);

impl PunctualShaderLight for DirectionalLightShaderInfo {
  type PunctualDependency = ();

  fn create_punctual_dep(
    _: &mut ShaderGraphFragmentBuilderView,
  ) -> Result<Self::PunctualDependency, ShaderGraphBuildError> {
    Ok(())
  }

  fn compute_incident_light(
    builder: &ShaderGraphFragmentBuilderView,
    light: &ENode<Self>,
    _dep: &Self::PunctualDependency,
    _ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderIncidentLight> {
    let shadow_info = light.shadow.expand();
    let occlusion = consts(0.).mutable();

    if_by(shadow_info.enabled, || {
      let map = builder.query::<BasicShadowMap>().unwrap();
      let sampler = builder.query::<BasicShadowMapSampler>().unwrap();
      let shadow_infos = builder.query::<BasicShadowMapInfoGroup>().unwrap();
      let shadow_info = shadow_infos.index(shadow_info.index).expand();

      // another way to compute this is in vertex shader, maybe we will try it later.
      let bias = shadow_info.bias.expand();
      let world_position = builder.query::<FragmentWorldPosition>().unwrap();
      let world_normal = builder.query::<FragmentWorldNormal>().unwrap();

      // apply normal bias
      let world_position = world_position + bias.normal_bias * world_normal;

      let shadow_position =
        shadow_info.shadow_camera.expand().view_projection_inv * (world_position, 1.).into();

      let shadow_position = shadow_position.xyz() / shadow_position.w();

      // convert to uv space and apply offset bias
      let shadow_position = shadow_position * consts(Vec3::new(0.5, 0.5, 1.))
        + consts(Vec3::new(1., 1., 0.))
        + (0., 0., bias.bias).into();

      let occlusion_result =
        directional_shadow_occlusion(shadow_position, map, sampler, shadow_info.map_info);

      occlusion.set(occlusion_result)
    });

    ENode::<ShaderIncidentLight> {
      color: light.intensity * (consts(1.) - occlusion.get()),
      direction: light.direction,
    }
  }
}

#[derive(Clone)]
struct DirectionalShadowGPU {
  shadow_camera: SceneCamera,
  map: ShadowMap,
}

fn get_shadow_map(
  inner: &SceneItemRefGuard<SceneLightInner<DirectionalLight>>,
  resources: &mut GPUResourceCache,
  shadows: &mut ShadowMapSystem,
) -> DirectionalShadowGPU {
  let resolution = Size::from_usize_pair_min_one((512, 512));

  resources
    .scene
    .lights
    .inner
    .entry(TypeId::of::<DirectionalLight>())
    .or_insert_with(|| {
      Box::new(IdentityMapper::<DirectionalShadowGPU, DirectionalLight>::default())
    })
    .as_any_mut()
    .downcast_mut::<IdentityMapper<DirectionalShadowGPU, DirectionalLight>>()
    .unwrap()
    .get_update_or_insert_with_logic(inner, |logic| match logic {
      ResourceLogic::Create(light) => {
        let shadow_camera = build_shadow_camera(light);
        let map = shadows.maps.allocate(resolution);
        ResourceLogicResult::Create(DirectionalShadowGPU { shadow_camera, map })
      }
      ResourceLogic::Update(shadow, light) => {
        let shadow_camera = build_shadow_camera(light);
        let map = shadows.maps.allocate(resolution);
        *shadow = DirectionalShadowGPU { shadow_camera, map };
        ResourceLogicResult::Update(shadow)
      }
    })
    .clone()
}

struct SceneDepth;

impl<S> PassContentWithSceneAndCamera<S> for SceneDepth
where
  S: SceneContent,
  S::Model: Deref<Target = dyn SceneModelShareable>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    let mut render_list = RenderList::<S>::default();
    render_list.prepare(scene, camera);

    // we could just use default, because the color channel not exist at all
    let base = pass.default_dispatcher();

    render_list.setup_pass(pass, scene, &base, camera);
  }
}

impl WebGPUSceneLight for SceneLight<DirectionalLight> {
  // allocate shadow maps
  fn pre_update(&self, ctx: &mut LightUpdateCtx) {
    let inner = self.read();
    get_shadow_map(&inner, ctx.ctx.resources, ctx.shadows);
  }

  fn update(&self, ctx: &mut LightUpdateCtx) {
    let inner = self.read();
    let light = &inner.light;
    let node = &inner.node;

    let DirectionalShadowGPU { shadow_camera, map } =
      get_shadow_map(&inner, ctx.ctx.resources, ctx.shadows);

    let (view, map_info) = map.get_write_view(ctx.ctx.gpu);
    let shadow_camera_info = ctx
      .ctx
      .resources
      .cameras
      .check_update_gpu(&shadow_camera, ctx.ctx.gpu)
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
      .or_insert_with(|| Box::new(BasicShadowMapInfoList::default()));
    let shadows = shadows
      .as_any_mut()
      .downcast_mut::<BasicShadowMapInfoList>()
      .unwrap();

    let index = shadows.list.source.len();

    let mut info = BasicShadowMapInfo::default();
    info.shadow_camera = shadow_camera_info;
    info.bias = ShadowBias::default();
    info.map_info = map_info;

    shadows.list.source.push(info);

    let lights = ctx.forward.get_or_create_list();
    let gpu = DirectionalLightShaderInfo {
      intensity: light.intensity,
      direction: node.get_world_matrix().forward().normalize().reverse(),
      shadow: LightShadowAddressInfo {
        enabled: false.into(),
        index: index as u32,
        ..Zeroable::zeroed()
      },
      ..Zeroable::zeroed()
    };

    lights.source.push(gpu)
  }
}

pub struct DirectionalShadowMapExtraInfo {
  pub width_extend: f32,
  pub height_extend: f32,
  pub up: Vec3<f32>,
}

fn build_shadow_camera(light: &SceneLightInner<DirectionalLight>) -> SceneCamera {
  let orth = OrthographicProjection {
    left: -20.,
    right: 20.,
    top: 20.,
    bottom: -20.,
    near: 0.1,
    far: 2000.,
  };
  let orth = WorkAroundResizableOrth { orth };
  SceneCamera::create_camera(orth, light.node.clone())
}

struct WorkAroundResizableOrth<T> {
  orth: OrthographicProjection<T>,
}

impl<T: Scalar> Projection<T> for WorkAroundResizableOrth<T> {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<T>) {
    self.orth.update_projection::<S>(projection);
  }

  fn pixels_per_unit(&self, distance: T, view_height: T) -> T {
    self.orth.pixels_per_unit(distance, view_height)
  }
}

impl<T: Scalar> ResizableProjection<T> for WorkAroundResizableOrth<T> {
  fn resize(&mut self, _size: (T, T)) {
    // nothing!
  }
}
impl HyperRayCaster<f32, Vec3<f32>, Vec2<f32>> for WorkAroundResizableOrth<f32> {
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> HyperRay<f32, Vec3<f32>> {
    self.orth.cast_ray(normalized_position)
  }
}
