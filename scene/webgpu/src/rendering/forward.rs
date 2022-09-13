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

pub struct ForwardScene;

impl<S> PassContentWithSceneAndCamera<S> for ForwardScene
where
  S: SceneContent,
  S::Model: Deref<Target = dyn SceneModelShareable>,
{
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &Scene<S>, camera: &SceneCamera) {
    let mut render_list = RenderList::<S>::default();
    render_list.prepare(scene, camera);
    render_list.setup_pass(pass, scene, &pass.default_dispatcher(), camera);
  }
}

/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
#[derive(Default)]
pub struct ForwardLightingSystem {
  pub lights_collections: LinkedHashMap<TypeId, Box<dyn ForwardLightCollection>>,
  light_hash_cache: u64,
}

impl ShaderPassBuilder for ForwardLightingSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for lights in self.lights_collections.values() {
      lights.setup_pass(ctx)
    }
  }
}

impl ShaderHashProvider for ForwardLightingSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.light_hash_cache.hash(hasher);
  }
}

impl ShaderGraphProvider for ForwardLightingSystem {
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.compute_lights(builder, &PhysicalShading)?;
    // todo get current shading
    // todo tonemap, write channel
    builder.fragment(|builder, _| {
      let ldr = builder.query::<LDRLightResult>()?;
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

impl ForwardLightingSystem {
  pub fn update_by_scene(&mut self, scene: &Scene<WebGPUScene>, gpu: &GPU) {
    self
      .lights_collections
      .iter_mut()
      .for_each(|(_, c)| c.reset());

    for (_, light) in &scene.lights {
      let light = &light.read().light;
      light.collect(self)
    }

    self
      .lights_collections
      .iter_mut()
      .for_each(|(_, c)| c.update_gpu(gpu));

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
      let camera_position = builder.query::<CameraWorldMatrix>()?.position();
      let geom_position = builder.query::<FragmentWorldPosition>()?;

      let geom_ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
        position: geom_position,
        normal: builder.query::<FragmentWorldNormal>()?,
        view_dir: camera_position - geom_position,
      };
      let shading = shading_impl.construct_shading_dyn(builder);

      let mut light_specular_result = consts(Vec3::zero());
      let mut light_diffuse_result = consts(Vec3::zero());

      for lights in self.lights_collections.values() {
        if lights.has_lights() {
          let (diffuse, specular) =
            lights.compute_lights(builder, binding, shading_impl, &shading, &geom_ctx)?;
          light_specular_result = specular + light_specular_result;
          light_diffuse_result = diffuse + light_diffuse_result;
        }
      }

      builder.register::<HDRLightResult>(light_diffuse_result + light_specular_result);

      Ok(())
    })
  }
}

#[derive(Default)]
pub struct LightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: Option<UniformBufferDataView<Shader140Array<T, 32>>>,
}

pub trait LightCollectionBase {
  fn reset(&mut self);
  fn update_gpu(&mut self, gpu: &GPU);
  fn has_lights(&self) -> bool;
}

impl<T: ShaderLight> LightCollectionBase for LightList<T> {
  fn reset(&mut self) {
    self.lights.clear();
    self.lights_gpu.take();
  }

  fn update_gpu(&mut self, gpu: &GPU) {
    let source: Vec<_> = self.lights.iter().copied().take(32).collect();
    let source = source.try_into().unwrap();
    let lights_gpu = UniformBufferDataResource::create_with_source(source, &gpu.device);
    let lights_gpu = lights_gpu.create_default_view();
    self.lights_gpu = lights_gpu.into();
  }

  fn has_lights(&self) -> bool {
    self.lights_gpu.is_some()
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

    for_by(lights, |_, light| {
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
