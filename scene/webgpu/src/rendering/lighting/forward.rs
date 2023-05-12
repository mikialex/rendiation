use crate::*;

pub struct ForwardScene<'a> {
  pub lights: &'a ForwardLightingSystem,
  pub shadow: &'a ShadowMapSystem,
  pub tonemap: &'a ToneMap,
  pub derives: &'a SceneNodeDeriveSystem,
  pub debugger: Option<&'a ScreenChannelDebugger>,
}

impl<'a> PassContentWithSceneAndCamera for ForwardScene<'a> {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    let mut render_list = RenderList::default();
    render_list.prepare(scene, camera);

    let base = default_dispatcher(pass);
    let dispatcher = ForwardSceneLightingDispatcher {
      base,
      lighting: self,
      debugger: self.debugger,
      override_shading: None,
      // override_shading: Some(&PhysicalShading),
    };

    render_list.setup_pass(pass, &dispatcher, camera, scene);
  }
}

pub struct ForwardSceneLightingDispatcher<'a> {
  base: DefaultPassDispatcher,
  lighting: &'a ForwardScene<'a>,
  override_shading: Option<&'static dyn LightableSurfaceShadingDyn>,
  debugger: Option<&'a ScreenChannelDebugger>,
}

const MAX_SUPPORT_LIGHT_KIND_COUNT: usize = 8;
/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
pub struct ForwardLightingSystem {
  pub lights_collections: HashMap<TypeId, Box<dyn ForwardLightCollection>>,
  /// note todo!, we don't support correct codegen for primitive wrapper array type
  /// so we use vec4<u32> instead of u32
  pub lengths:
    Option<UniformBufferDataView<Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT>>>,
  light_hash_cache: u64,
}

impl ForwardLightingSystem {
  pub fn new(scene: &Scene, shadow: &ShadowMapSystem) -> Self {
    scene
      .unbound_listen_by(|view, send| match view {
        MaybeDeltaRef::All(scene) => scene.lights.expand(send),
        MaybeDeltaRef::Delta(delta) => {
          if let SceneInnerDelta::lights(d) = delta {
            send(d.clone())
          }
        }
      })
      .map(|d| {
        //
      })
  }
}

impl<'a> ShaderPassBuilder for ForwardSceneLightingDispatcher<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.lighting.shadow.setup_pass(ctx);

    ctx
      .binding
      .bind(self.lighting.lights.lengths.as_ref().unwrap(), SB::Pass);
    for lights in self.lighting.lights.lights_collections.values() {
      lights.setup_pass(ctx)
    }
    self.lighting.tonemap.setup_pass(ctx);
  }
}

impl<'a> ShaderHashProvider for ForwardSceneLightingDispatcher<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.lighting.lights.light_hash_cache.hash(hasher);
    self.lighting.shadow.hash_pipeline(hasher);

    self.debugger.is_some().hash(hasher);
    if let Some(debugger) = &self.debugger {
      debugger.hash_pipeline(hasher);
    }

    self.override_shading.type_id().hash(hasher);
  }
}

impl<'a> ShaderHashProviderAny for ForwardSceneLightingDispatcher<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.hash_pipeline(hasher);
    // this is so special(I think) that id could skip
  }
}

pub struct ShadingSelection;

impl<'a> ShaderGraphProvider for ForwardSceneLightingDispatcher<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.base.build(builder)
  }
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.lighting.shadow.build(builder)?;

    let shading_impl = if let Some(override_shading) = self.override_shading {
      override_shading
    } else {
      *builder
        .context
        .entry(ShadingSelection.type_id())
        .or_insert_with(|| Box::new(&PhysicalShading as &dyn LightableSurfaceShadingDyn))
        .downcast_ref::<&dyn LightableSurfaceShadingDyn>()
        .unwrap()
    };

    self.lighting.lights.compute_lights(builder, shading_impl)?;

    self.lighting.tonemap.build(builder)?;

    builder.fragment(|builder, _| {
      let ldr = builder.query::<LDRLightResult>()?;

      let alpha = builder
        .query::<AlphaChannel>()
        .unwrap_or_else(|_| consts(1.0));

      // should we use other way to get mask mode?
      let alpha = if builder.query::<AlphaCutChannel>().is_ok() {
        if_by(alpha.equals(consts(0.)), || builder.discard());
        consts(1.)
      } else {
        alpha
      };

      builder.set_fragment_out(0, (ldr, alpha))
    })?;

    if let Some(debugger) = &self.debugger {
      debugger.build(builder)?;
    }
    Ok(())
  }
}

pub trait ForwardLightCollection:
  LightCollectionCompute + RebuildAbleGPUCollectionBase + Any
{
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: LightCollectionCompute + RebuildAbleGPUCollectionBase + Any> ForwardLightCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

// a little bit hack
only_fragment!(LightCount, u32);

pub trait BuilderUsefulExt {
  fn get_or_compute_fragment_normal(&mut self) -> Node<Vec3<f32>>;
}

impl<'a> BuilderUsefulExt for ShaderGraphFragmentBuilderView<'a> {
  fn get_or_compute_fragment_normal(&mut self) -> Node<Vec3<f32>> {
    // check first and avoid unnecessary renormalize
    if let Ok(normal) = self.query::<FragmentWorldNormal>() {
      normal
    } else {
      let normal = self.query_or_interpolate_by::<FragmentWorldNormal, WorldVertexNormal>();
      let normal = normal.normalize(); // renormalize
      self.register::<FragmentWorldNormal>(normal);
      normal
    }
  }
}

impl ForwardLightingSystem {
  pub fn get_or_create_list<T: ShaderLight>(&mut self) -> &mut LightList<T> {
    let lights = self
      .lights_collections
      .entry(TypeId::of::<T>())
      .or_insert_with(|| Box::new(LightList::<T>::default_with(SB::Pass)));
    lights.as_any_mut().downcast_mut::<LightList<T>>().unwrap()
  }

  pub fn before_update_scene(&mut self, _gpu: &GPU) {
    self
      .lights_collections
      .iter_mut()
      .for_each(|(_, c)| c.reset());
  }

  pub fn after_update_scene(&mut self, gpu: &GPU) {
    let mut lengths: Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT> = Default::default();

    self
      .lights_collections
      .iter_mut()
      .map(|(_, c)| c.update_gpu(gpu))
      .enumerate()
      .for_each(|(i, l)| lengths.inner[i] = Vec4::new(l as u32, 0, 0, 0).into());

    self.lengths = create_uniform(lengths, gpu).into();

    let mut hasher = PipelineHasher::default();
    for lights in self.lights_collections.values() {
      lights.hash_pipeline(&mut hasher)
    }
    self.light_hash_cache = hasher.finish();
  }

  pub fn compute_lights(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let lengths_info = binding.uniform_by(self.lengths.as_ref().unwrap(), SB::Pass);
      let camera_position = builder.query::<CameraWorldMatrix>()?.position();
      let position =
        builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();
      let normal = builder.get_or_compute_fragment_normal();

      let geom_ctx = ENode::<ShaderLightingGeometricCtx> {
        position,
        normal,
        view_dir: (camera_position - position).normalize(),
      };
      let shading = shading_impl.construct_shading_dyn(builder);

      let mut light_specular_result = consts(Vec3::zero());
      let mut light_diffuse_result = consts(Vec3::zero());

      for (i, lights) in self.lights_collections.values().enumerate() {
        let length = lengths_info.index(consts(i as u32)).x();
        builder.register::<LightCount>(length);

        let (diffuse, specular) =
          lights.compute_lights(builder, binding, shading_impl, shading.as_ref(), &geom_ctx)?;
        light_specular_result = specular + light_specular_result;
        light_diffuse_result = diffuse + light_diffuse_result;
      }

      builder.register::<HDRLightResult>(light_diffuse_result + light_specular_result);

      Ok(())
    })
  }
}

const LIGHT_MAX: usize = 8;
pub type LightList<T> = ClampedUniformList<T, LIGHT_MAX>;

impl<T: ShaderLight> RebuildAbleGPUCollectionBase for LightList<T> {
  fn reset(&mut self) {
    self.reset()
  }

  fn update_gpu(&mut self, gpu: &GPU) -> usize {
    self.update_gpu(gpu)
  }
}

pub trait LightCollectionCompute: ShaderPassBuilder + ShaderHashProvider {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError>;

  fn compute_lights_grouped(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<ENode<ShaderLightingResult>, ShaderGraphBuildError> {
    let (diffuse, specular) =
      self.compute_lights(builder, binding, shading_impl, shading, geom_ctx)?;
    Ok(ENode::<ShaderLightingResult> { diffuse, specular })
  }
}

impl<T: ShaderLight> LightCollectionCompute for LightList<T> {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError> {
    let lights = binding.uniform_by(self.gpu.as_ref().unwrap(), SB::Pass);

    let dep = T::create_dep(builder)?;

    let light_specular_result = consts(Vec3::zero()).mutable();
    let light_diffuse_result = consts(Vec3::zero()).mutable();

    let light_count = builder.query::<LightCount>()?;

    let light_iter = ClampedShaderIter {
      source: lights,
      count: light_count,
    };

    for_by_ok(light_iter, |_, light, _| {
      let light = light.expand();
      let light_result =
        T::compute_direct_light(builder, &light, geom_ctx, shading_impl, shading, &dep)?;

      // improve impl by add assign
      light_specular_result.set(light_specular_result.get() + light_result.specular);
      light_diffuse_result.set(light_diffuse_result.get() + light_result.diffuse);
      Ok(())
    })?;

    Ok((light_diffuse_result.get(), light_specular_result.get()))
  }
}
