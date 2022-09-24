use crate::*;

pub fn get_main_pass_load_op<S>(scene: &Scene<S>) -> webgpu::Operations<webgpu::Color>
where
  S: SceneContent,
  S::BackGround: Deref<Target = dyn WebGPUBackground>,
{
  let load = if let Some(clear_color) = scene.background.as_ref().unwrap().require_pass_clear() {
    webgpu::LoadOp::Clear(clear_color)
  } else {
    webgpu::LoadOp::Load
  };

  webgpu::Operations { load, store: true }
}

pub struct ForwardScene<'a> {
  pub lights: &'a ForwardLightingSystem,
  pub tonemap: &'a ToneMap,
}

impl<'a, S> PassContentWithSceneAndCamera<S> for ForwardScene<'a>
where
  S: SceneContent,
  S::Model: Deref<Target = dyn SceneModelShareable>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    let mut render_list = RenderList::<S>::default();
    render_list.prepare(scene, camera);

    let base = pass.default_dispatcher();
    let dispatcher = ForwardSceneLightingDispatcher {
      base,
      lighting: self,
      override_shading: Some(Rc::new(PhysicalShading)),
    };

    render_list.setup_pass(pass, scene, &dispatcher, camera);
  }
}

pub struct ForwardSceneLightingDispatcher<'a> {
  base: DefaultPassDispatcher,
  lighting: &'a ForwardScene<'a>,
  override_shading: Option<Rc<dyn LightableSurfaceShadingDyn>>,
}

const MAX_SUPPORT_LIGHT_KIND_COUNT: usize = 8;
/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
#[derive(Default)]
pub struct ForwardLightingSystem {
  pub lights_collections: LinkedHashMap<TypeId, Box<dyn ForwardLightCollection>>,
  /// note todo!, we don't support correct codegen for primitive wrapper array type
  /// so we use vec4<u32> instead of u32
  pub lengths:
    Option<UniformBufferDataView<Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT>>>,
  light_hash_cache: u64,
}

impl<'a> ShaderPassBuilder for ForwardSceneLightingDispatcher<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
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
    let shading_impl = builder
      .context
      .entry(ShadingSelection.type_id())
      .or_insert_with(|| {
        if let Some(override_shading) = &self.override_shading {
          Box::new(override_shading.clone())
        } else {
          Box::new(Rc::new(PhysicalShading))
        }
      })
      .downcast_ref::<Rc<dyn LightableSurfaceShadingDyn>>()
      .unwrap()
      .clone();

    self
      .lighting
      .lights
      .compute_lights(builder, shading_impl.as_ref())?;

    self.lighting.tonemap.build(builder)?;

    builder.fragment(|builder, _| {
      let ldr = builder.query::<HDRLightResult>()?;

      // // normal debug
      // let normal = builder.query::<FragmentWorldNormal>()?;
      // let normal = (normal + consts(Vec3::one())) * consts(0.5);
      // builder.set_fragment_out(0, (normal, 1.))

      builder.set_fragment_out(0, (ldr, 1.))
    })
  }
}

pub trait ForwardLightCollection: LightCollectionCompute + LightCollectionBase + Any {
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: LightCollectionCompute + LightCollectionBase + Any> ForwardLightCollection for T {
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

wgsl_fn!(
  fn compute_normal_by_dxdy(position: vec3<f32>) -> vec3<f32> {
    /// note, webgpu canvas is left handed
    return normalize(cross(dpdy(position), dpdx(position)));
  }
);

// a little bit hack
only_fragment!(LightCount, u32);

impl ForwardLightingSystem {
  pub fn update_by_scene(&mut self, scene: &Scene<WebGPUScene>, gpu: &GPU) {
    self
      .lights_collections
      .iter_mut()
      .for_each(|(_, c)| c.reset());

    for (_, light) in &scene.lights {
      let light = &light.read();
      light.collect(self)
    }

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
      let normal = builder.query_or_interpolate_by::<FragmentWorldNormal, WorldVertexNormal>();
      builder.register::<FragmentWorldNormal>(normal.normalize()); // renormalize

      // debug
      // let normal = compute_normal_by_dxdy(position);
      // builder.register::<FragmentWorldNormal>(normal);

      let geom_ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
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

#[derive(Default)]
pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: Option<UniformBufferDataView<Shader140Array<T, LIGHT_MAX>>>,
}

pub trait LightCollectionBase {
  fn reset(&mut self);
  /// return count
  fn update_gpu(&mut self, gpu: &GPU) -> usize;
}

impl<T: ShaderLight + Default> LightCollectionBase for LightList<T> {
  fn reset(&mut self) {
    self.lights.clear();
    self.lights_gpu.take();
  }

  fn update_gpu(&mut self, gpu: &GPU) -> usize {
    let mut source = vec![T::default(); LIGHT_MAX];
    for (i, light) in self.lights.iter().enumerate() {
      if i >= LIGHT_MAX {
        break;
      }
      source[i] = *light;
    }
    let source = source.try_into().unwrap();
    let lights_gpu = create_uniform(source, gpu);
    self.lights_gpu = lights_gpu.into();
    self.lights.len()
  }
}

impl<T: ShaderLight> ShaderPassBuilder for LightList<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .binding
      .bind(self.lights_gpu.as_ref().unwrap(), SB::Pass);
  }
}

impl<T: ShaderLight> ShaderHashProvider for LightList<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<T>().hash(hasher);
    self.lights.len().hash(hasher);
  }
}

pub trait LightCollectionCompute: ShaderPassBuilder + ShaderHashProvider {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError>;

  fn compute_lights_grouped(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<ExpandedNode<ShaderLightingResult>, ShaderGraphBuildError> {
    let (diffuse, specular) =
      self.compute_lights(builder, binding, shading_impl, shading, geom_ctx)?;
    Ok(ExpandedNode::<ShaderLightingResult> { diffuse, specular })
  }
}

impl<T: ShaderLight> LightCollectionCompute for LightList<T> {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError> {
    let lights = binding.uniform_by(self.lights_gpu.as_ref().unwrap(), SB::Pass);

    let dep = T::create_dep(builder);

    let light_specular_result = consts(Vec3::zero()).mutable();
    let light_diffuse_result = consts(Vec3::zero()).mutable();

    let light_count = builder.query::<LightCount>()?;

    let light_iter = ClampedShaderIter {
      inner: lights,
      count: light_count,
    };

    for_by(light_iter, |_, light, _| {
      let light = light.expand();
      let incident = T::compute_direct_light(&light, &dep, geom_ctx);
      let light_result = shading_impl.compute_lighting_dyn(shading, &incident, geom_ctx);

      // improve impl by add assign
      light_specular_result.set(light_specular_result.get() + light_result.specular);
      light_diffuse_result.set(light_diffuse_result.get() + light_result.diffuse);
    });

    Ok((light_diffuse_result.get(), light_specular_result.get()))
  }
}
