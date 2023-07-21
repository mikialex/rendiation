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
  AsRef<dyn LightCollectionCompute> + Stream<Item = usize> + Unpin
{
}
impl<T> ReactiveLightCollectionCompute for T where
  T: AsRef<dyn LightCollectionCompute> + Stream<Item = usize> + Unpin
{
}

const MAX_SUPPORT_LIGHT_KIND_COUNT: usize = 8;

type LightCollections = StreamMap<TypeId, Box<dyn ReactiveLightCollectionCompute>>;

/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
#[pin_project::pin_project]
pub struct ForwardLightingSystem {
  gpu: ResourceGPUCtx,
  /// note, the correctness now actually rely on the hashmap in stream map provide stable iter in
  /// stable order. currently, as long as we not insert new collection in runtime, it holds.
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
    let r = if let Poll::Ready(Some(updates)) = this.lights_collections.poll_next_unpin(cx) {
      for update in updates {
        if let StreamMapDelta::Delta(tid, new_len) = update {
          let index = this.mapping_length_idx.get(&tid).unwrap();
          this.lengths.mutate(|lengths| {
            lengths.inner[*index] = Vec4::new(new_len as u32, 0, 0, 0).into();
          });
        }
      }
      this.lengths.upload_with_diff(&this.gpu.queue);
      Poll::Ready(().into())
    } else {
      Poll::Pending
    };

    use std::hash::Hasher;
    let mut hasher = PipelineHasher::default();
    for lights in this.lights_collections.values() {
      lights.as_ref().as_ref().hash_pipeline(&mut hasher)
    }
    *this.light_hash_cache = hasher.finish();

    r
  }
}

impl ForwardLightingSystem {
  pub fn new(scene: &Scene, gpu: ResourceGPUCtx, res: LightResourceCtx) -> Self {
    let lights_collections = LightCollections::default();

    let scene_light_change = scene
      .unbound_listen_by(all_delta)
      .filter_map_sync(|d| match d {
        MixSceneDelta::lights(l) => l.into(),
        _ => None,
      })
      .map(|light| match light {
        ContainerRefRetainContentDelta::Remove(light) => (light.guid(), None),
        ContainerRefRetainContentDelta::Insert(light) => (light.guid(), Some(light)),
      })
      .create_broad_caster();

    let mut collections = StreamMap::default();

    let spot = scene_light_change
      .fork_stream()
      .map(|(light_id, light)| {
        let light_stream = light.map(|light| {
          let light_weak = light.downgrade();
          let res = res.clone();
          light
            .single_listen_by(with_field!(SceneLightInner => light))
            .map(|l: SceneLightKind| match l {
              SceneLightKind::SpotLight(l) => Some(l),
              _ => None,
            })
            .map(move |l| {
              light_weak.upgrade().zip(l).map(|(light, light_ty)| {
                light_ty.create_uniform_stream(
                  &res,
                  Box::new(light.single_listen_by(with_field!(SceneLightInner => node))),
                )
              })
            })
            .flatten_option_outer()
        });
        (light_id, light_stream)
      })
      .flatten_into_map_stream_signal()
      .map(|updates| {
        updates
          .into_iter()
          .filter_map(|update| match update {
            StreamMapDelta::Insert(_) => None,
            StreamMapDelta::Remove(id) => Some((id, None)),
            StreamMapDelta::Delta(id, v) => Some((id, v)),
          })
          .collect()
      })
      .merge_into_light_list(gpu.clone());

    collections.insert(TypeId::of::<SpotLight>(), Box::new(spot));

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
    for lights in self.lights.lights_collections.values() {
      lights.as_ref().as_ref().setup_pass(ctx)
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

      for (i, lights) in self.lights_collections.values().enumerate() {
        let length = lengths_info.index(consts(i as u32)).x();
        builder.register::<LightCount>(length);

        let (diffuse, specular) = lights.as_ref().as_ref().compute_lights(
          builder,
          binding,
          shading_impl,
          shading.as_ref(),
          &geom_ctx,
        )?;
        light_specular_result = specular + light_specular_result;
        light_diffuse_result = diffuse + light_diffuse_result;
      }

      builder.register::<HDRLightResult>(light_diffuse_result + light_specular_result);

      Ok(())
    })
  }
}

#[pin_project::pin_project]
struct ReactiveLightList<S, T: ShaderLight> {
  list: LightList<T>,
  #[pin]
  input: S,
}

impl<S, T: ShaderLight> AsRef<dyn LightCollectionCompute> for ReactiveLightList<S, T> {
  fn as_ref(&self) -> &(dyn LightCollectionCompute + 'static) {
    &self.list
  }
}

trait StreamForLightExt: Sized + Stream {
  fn flatten_option_outer<SS: Stream>(self) -> FlattenOptionOuter<Self, SS>
  where
    Self: Stream<Item = Option<SS>>;

  fn merge_into_light_list<T: ShaderLight>(self, gpu: ResourceGPUCtx) -> ReactiveLightList<Self, T>
  where
    Self: Stream<Item = Vec<(usize, Option<T>)>>;
}
impl<T: Sized + Stream> StreamForLightExt for T {
  fn flatten_option_outer<SS>(self) -> FlattenOptionOuter<Self, SS>
  where
    Self: Stream<Item = Option<SS>>,
    SS: Stream,
  {
    FlattenOptionOuter {
      stream: self,
      next: None,
    }
  }

  fn merge_into_light_list<TT: ShaderLight>(
    self,
    gpu: ResourceGPUCtx,
  ) -> ReactiveLightList<Self, TT>
  where
    Self: Stream<Item = Vec<(usize, Option<TT>)>>,
  {
    ReactiveLightList {
      list: LightList::<TT>::new(gpu),
      input: self,
    }
  }
}
#[pin_project::pin_project]
struct FlattenOptionOuter<S, SS> {
  #[pin]
  stream: S,
  #[pin]
  next: Option<Option<SS>>,
}

impl<S, SS> Stream for FlattenOptionOuter<S, SS>
where
  S: Stream<Item = Option<SS>>,
  SS: Stream,
{
  type Item = Option<SS::Item>;
  /// first we check the outside, if the option yields, we pending, the other behavior is as same as
  /// the flatten signal in reactive crate
  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut this = self.project();
    Poll::Ready(loop {
      // compare to the flatten, we poll the outside stream first
      if let Poll::Ready(Some(s)) = this.stream.as_mut().poll_next(cx) {
        this.next.set(Some(s));
      } else if let Some(mut s) = this.next.as_mut().as_pin_mut() {
        if let Some(s) = s.as_mut().as_pin_mut() {
          if let Some(item) = ready!(s.poll_next(cx)) {
            break Some(Some(item));
          } else {
            this.next.set(None);
          }
        } else {
          return Poll::Pending;
        }
      } else {
        break None;
      }
    })
  }
}

impl<T, S> Stream for ReactiveLightList<S, T>
where
  T: ShaderLight,
  S: Stream<Item = Vec<(usize, Option<T>)>>,
{
  type Item = usize;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(Some(updates)) = this.input.poll_next(cx) {
      for (light_id, light) in updates {
        this.list.update(light_id, light);
      }
      if let Some(new_len) = this.list.maintain() {
        Poll::Ready(new_len.into())
      } else {
        Poll::Pending
      }
    } else {
      Poll::Pending
    }
  }
}

const LIGHT_MAX: usize = 8;

pub struct LightList<T: ShaderLight> {
  uniform: ClampedUniformList<T, LIGHT_MAX>,
  empty_list: Vec<usize>,
  // map light id to index
  mapping: FastHashMap<usize, usize>,
  gpu: ResourceGPUCtx,
}

impl<T: ShaderLight> LightList<T> {
  pub fn new(gpu: ResourceGPUCtx) -> Self {
    Self {
      uniform: Default::default(),
      empty_list: (0..LIGHT_MAX).rev().collect(),
      mapping: Default::default(),
      gpu,
    }
  }

  pub fn update(&mut self, light_id: usize, light: Option<T>) {
    if let Some(value) = light {
      let idx = self.empty_list.pop().unwrap();
      self.mapping.insert(light_id, idx);
      self.uniform.source[idx] = value;
    } else {
      let idx = self.mapping.remove(&light_id).unwrap();
      self.empty_list.push(idx);
    }
  }

  pub fn maintain(&mut self) -> Option<usize> {
    let empty_size = self.empty_list.len();

    // self.empty_list.sort_by(|a, b| b.cmp(a));

    // for i in 0..empty_size {
    //   let check_idx = LIGHT_MAX - 1 - i;
    //   if let Err(insert_position) = self
    //     .empty_list
    //     .binary_search_by(|a| check_idx.cmp(a)) // because we're reverse sort

    //   {
    //     let target = self.empty_list.pop().unwrap();
    //     self.uniform.source[target] = self.uniform.source[check_idx];
    //     self.empty_list.insert(insert_position - 1, element)
    //   }
    // }

    self.uniform.update_gpu(&self.gpu.device);
    todo!();
    Some(LIGHT_MAX - empty_size) // todo
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
