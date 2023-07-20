use crate::*;

pub fn get_main_pass_load_op(scene: &SceneCoreImpl) -> webgpu::Operations<webgpu::Color> {
  let load = if let Some(bg) = &scene.background {
    if let Some(clear_color) = match bg {
      SceneBackGround::Solid(bg) => bg.require_pass_clear(),
      SceneBackGround::Env(bg) => bg.require_pass_clear(),
      SceneBackGround::Foreign(bg) => {
        if let Some(bg) = bg.downcast_ref::<Box<dyn WebGPUBackground>>() {
          bg.require_pass_clear()
        } else {
          None
        }
      }
      _ => None,
    } {
      webgpu::LoadOp::Clear(clear_color)
    } else {
      webgpu::LoadOp::Load
    }
  } else {
    webgpu::LoadOp::Load
  };

  webgpu::Operations { load, store: true }
}

pub struct ForwardScene<'a> {
  pub tonemap: &'a ToneMap,
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
      lights: &scene.scene_resources.lights,
      shadows: &scene.scene_resources.shadows,
    };

    render_list.setup_pass(pass, &dispatcher, camera, scene);
  }
}

pub struct ForwardSceneLightingDispatcher<'a> {
  base: DefaultPassDispatcher,
  lighting: &'a ForwardScene<'a>,
  lights: &'a ForwardLightingSystem,
  shadows: &'a ShadowMapSystem,
  override_shading: Option<&'static dyn LightableSurfaceShadingDyn>,
  debugger: Option<&'a ScreenChannelDebugger>,
}

pub trait ReactiveLightCollectionCompute:
  LightCollectionCompute + Stream<Item = usize> + Any
{
  // fn insert(&mut self, light_id: usize, i)
}

const MAX_SUPPORT_LIGHT_KIND_COUNT: usize = 8;

type LightCollections = Arc<RwLock<StreamMap<TypeId, Box<dyn ReactiveLightCollectionCompute>>>>;

/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
#[pin_project::pin_project]
pub struct ForwardLightingSystem {
  gpu: ResourceGPUCtx,
  pub lights_collections: LightCollections,
  // we could use linked hashmap to keep visit order
  pub mapping_length_idx: FastHashMap<TypeId, usize>,

  /// note todo!, we don't support correct codegen for primitive wrapper array type
  /// so we use vec4<u32> instead of u32
  pub lengths: UniformBufferDataView<Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT>>,

  light_hash_cache: u64,
}

impl Stream for ForwardLightingSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    // this.lights_collections.

    //   let mut lengths: Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT> =
    // Default::default();

    //   self
    //     .lights_collections
    //     .iter_mut()
    //     .map(|(_, c)| c.update_gpu(gpu))
    //     .enumerate()
    //     .for_each(|(i, l)| lengths.inner[i] = Vec4::new(l as u32, 0, 0, 0).into());

    //   self.lengths = create_uniform(lengths, gpu).into();

    //   let mut hasher = PipelineHasher::default();
    //   for lights in self.lights_collections.values() {
    //     lights.hash_pipeline(&mut hasher)
    //   }
    //   self.light_hash_cache = hasher.finish();

    Poll::Pending
  }
}

impl ForwardLightingSystem {
  pub fn new(scene: &Scene, gpu: ResourceGPUCtx) -> Self {
    fn insert_light(c: &LightCollections, light: SceneLight) {
      let mut collection = c.write().unwrap();

      let light_impl_change = light
        .single_listen_by(with_field!(SceneLightInner => light))
        .create_broad_caster();

      light_impl_change
        .fork_stream()
        .map(|l: SceneLightKind| match l {
          SceneLightKind::SpotLight(l) => Enable(l.into()),
          _ => Disable,
        })
        .map(|l| {
          l.create_uniform_stream(
            todo!(),
            Box::new(light_weak.single_listen_by(with_field!(SceneLightInner => node))),
          )
        }) // note we not use fork because we want init value,
        .flatten_signal()

      // let node = Box::new(node);
      // let light = light.read();
      // match &light.light {
      //   SceneLightKind::PointLight(_) => todo!(),
      //   SceneLightKind::SpotLight(light) => {
      //     let uniform = light.create_uniform_stream(todo!(), node);

      //     //
      //   }
      //   SceneLightKind::DirectionalLight(light) => {
      //     let uniform = light.create_uniform_stream(todo!(), node);
      //   }
      //   SceneLightKind::Foreign(_) => todo!(),
      //   _ => todo!(),
      // }
    }

    fn remove_light(c: &LightCollections, light: SceneLight) {
      let light = light.read();
      let mut collection = c.write().unwrap();
      match &light.light {
        SceneLightKind::PointLight(_) => todo!(),
        SceneLightKind::SpotLight(light) => {
          // collection.get_mut(TypeId::of::<SpotLight>)
        }
        SceneLightKind::DirectionalLight(light) => {}
        SceneLightKind::Foreign(_) => todo!(),
        _ => todo!(),
      }
    }

    let lights_collections = LightCollections::default();

    let lc = lights_collections.clone();

    let updater = scene.unbound_listen_by(all_delta).map(|d| match d {
      MixSceneDelta::lights(l) => {
        use ContainerRefRetainContentDelta::*;
        match l {
          Remove(l) => remove_light(&lc, l),
          Insert(l) => insert_light(&lc, l),
        }
      }
      _ => {}
    });

    let lengths = create_uniform2(Default::default(), &gpu.device);

    Self {
      gpu,
      lengths,
      lights_collections,
      mapping_length_idx: Default::default(),
      light_hash_cache: Default::default(),
    }
  }
}

impl<'a> ShaderPassBuilder for ForwardSceneLightingDispatcher<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.shadows.setup_pass(ctx);

    ctx.binding.bind(&self.lights.lengths);
    let lights_collections = self.lights.lights_collections.read().unwrap();
    for lights in lights_collections.values() {
      lights.setup_pass(ctx)
    }
    self.lighting.tonemap.setup_pass(ctx);
  }
}

impl<'a> ShaderHashProvider for ForwardSceneLightingDispatcher<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.lights.light_hash_cache.hash(hasher);
    self.shadows.hash_pipeline(hasher);

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
    self.shadows.build(builder)?;

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

    self.lights.compute_lights(builder, shading_impl)?;

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

// a little bit hack
only_fragment!(LightCount, u32);

impl ForwardLightingSystem {
  pub fn compute_lights(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let lengths_info = binding.uniform_by(&self.lengths);
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

      let lights_collections = self.lights_collections.read().unwrap();
      for (i, lights) in lights_collections.values().enumerate() {
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

#[pin_project::pin_project]
pub struct LightList<T: ShaderLight> {
  uniform: ClampedUniformList<T, LIGHT_MAX>,
  empty_list: Vec<usize>,
  mapping: FastHashMap<usize, usize>,
  source: StreamMap<usize, Box<dyn Stream<Item = T> + Unpin>>,
  gpu: ResourceGPUCtx,
}

impl<T: ShaderLight> LightList<T> {
  pub fn new(gpu: ResourceGPUCtx) -> Self {
    Self {
      uniform: Default::default(),
      empty_list: (0..LIGHT_MAX).collect(),
      mapping: Default::default(),
      source: Default::default(),
      gpu,
    }
  }

  pub fn insert_light(&mut self, light_id: usize, light: impl Stream<Item = T> + Unpin + 'static) {
    let idx = self.empty_list.pop().unwrap();
    self.mapping.insert(light_id, idx);
    self.source.insert(light_id, Box::new(light));
  }
}

impl<T: ShaderLight> Stream for LightList<T> {
  type Item = usize;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.source.loop_poll_until_pending(cx, |updates| {
      for update in updates {
        match update {
          StreamMapDelta::Remove(id) => {
            let idx = this.mapping.remove(&id).unwrap();
            this.empty_list.push(idx);
          }
          StreamMapDelta::Delta(id, value) => {
            let idx = this.mapping.get(&id).unwrap();
            this.uniform.source[*idx] = value;
          }
          _ => {}
        }
      }
    });

    this.uniform.update_gpu(&this.gpu.device);
    todo!();
    Poll::Pending
  }
}

impl<T: ShaderLight> ShaderHashProvider for LightList<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.uniform.hash_pipeline(hasher)
  }
}
impl<T: ShaderLight> ShaderPassBuilder for LightList<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.uniform.setup_pass(ctx)
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.uniform.post_setup_pass(ctx)
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
    let lights = binding.uniform_by(self.uniform.gpu.as_ref().unwrap());

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
