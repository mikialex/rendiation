use futures::stream::FusedStream;
use rendiation_shader_library::normal_mapping::BuilderNormalExt;
use rendiation_texture_gpu_process::ToneMap;

use crate::*;

pub fn get_main_pass_load_op(scene: &SceneCoreImpl) -> Operations<Color> {
  let load = if let Some(bg) = &scene.background {
    if let Some(clear_color) = match bg {
      SceneBackGround::Solid(bg) => bg.require_pass_clear(),
      SceneBackGround::Env(bg) => bg.require_pass_clear(),
      SceneBackGround::Foreign(bg) => {
        if let Some(bg) =
          get_dyn_trait_downcaster_static!(WebGPUBackground).downcast_ref(bg.as_ref().as_any())
        {
          bg.require_pass_clear()
        } else {
          None
        }
      }
    } {
      LoadOp::Clear(clear_color)
    } else {
      LoadOp::Load
    }
  } else {
    LoadOp::Load
  };

  Operations {
    load,
    store: StoreOp::Store,
  }
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
    render_list.collect_from_scene(scene, camera, true);
    let render_list = MaybeBindlessMeshRenderList::from_list(render_list, scene);

    let base = default_dispatcher(pass);
    let dispatcher = ForwardSceneLightingDispatcher {
      base,
      lighting: self,
      debugger: self.debugger,
      override_shading: None,
      lights: &scene.scene_resources.lights,
      shadows: &scene.scene_resources.shadows,
    };
    let dispatcher =
      &scene.extend_bindless_resource_provider(&dispatcher) as &dyn RenderComponentAny;

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

/// contains gpu data that support forward rendering
///
/// all uniform is update once in a frame. for convenience.
#[pin_project::pin_project]
pub struct ForwardLightingSystem {
  gpu: ResourceGPUCtx,
  /// note, the correctness now actually rely on the hashmap in stream map provide stable iter in
  /// stable order. currently, as long as we not insert new collection in runtime, it holds.
  pub lights_collections: StreamMap<TypeId, Box<dyn ReactiveLightCollectionCompute>>,
  // we could use linked hashmap to keep visit order
  pub mapping_length_idx: FastHashMap<TypeId, usize>,
  pub lights_insert_order: Vec<TypeId>,

  /// note todo!, we don't support correct codegen for primitive wrapper array type
  /// so we use vec4<u32> instead of u32
  pub lengths: UniformBufferCachedDataView<Shader140Array<Vec4<u32>, MAX_SUPPORT_LIGHT_KIND_COUNT>>,

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
    for lid in this.lights_insert_order {
      let lights = this.lights_collections.get(lid).unwrap();
      lights.as_ref().as_ref().hash_pipeline(&mut hasher)
    }
    *this.light_hash_cache = hasher.finish();

    r
  }
}
impl FusedStream for ForwardLightingSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}

impl ForwardLightingSystem {
  pub fn new(scene: &Scene, gpu: ResourceGPUCtx, res: LightResourceCtx) -> Self {
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

    let mut lights_collections = StreamMap::default();
    let mut mapping_length_idx = FastHashMap::default();
    let mut lights_insert_order = Vec::default();

    fn register_light_ty<T>(
      lights_collections: &mut StreamMap<TypeId, Box<dyn ReactiveLightCollectionCompute>>,
      mapping_length_idx: &mut FastHashMap<TypeId, usize>,
      lights_insert_order: &mut Vec<TypeId>,
      gpu: &ResourceGPUCtx,
      res: &LightResourceCtx,
      upstream: impl Stream<Item = (u64, Option<SceneLight>)> + Unpin + 'static,
      downcaster: impl Fn(LightEnum) -> Option<T> + Copy + 'static,
    ) where
      T: WebGPULight,
      T::Uniform: ShaderLight,
    {
      let res = res.clone();
      let light_impl = upstream
        .map(move |(light_id, light)| {
          let light_stream = light.map(|light| {
            let light_weak = light.downgrade();
            let res = res.clone();
            light
              .single_listen_by(with_field!(SceneLightImpl => light))
              .map(downcaster)
              .map(move |l| {
                light_weak.upgrade().zip(l).map(|(light, light_ty)| {
                  light_ty.create_uniform_stream(
                    &res,
                    Box::new(light.single_listen_by(with_field!(SceneLightImpl => node))),
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

      let tid = TypeId::of::<T>();
      let li = Box::new(light_impl) as Box<dyn ReactiveLightCollectionCompute>;
      lights_collections.insert(tid, li);
      mapping_length_idx.insert(tid, mapping_length_idx.len());
      lights_insert_order.push(tid);
    }

    macro_rules! register_light_ty {
      ($Type: tt) => {
        register_light_ty(
          &mut lights_collections,
          &mut mapping_length_idx,
          &mut lights_insert_order,
          &gpu,
          &res,
          scene_light_change.fork_stream(),
          |l| match l {
            LightEnum::$Type(l) => Some(l),
            _ => None,
          },
        );
      };
    }

    register_light_ty!(SpotLight);
    register_light_ty!(DirectionalLight);
    register_light_ty!(PointLight);

    let lengths = create_uniform_with_cache(Default::default(), &gpu.device);

    Self {
      gpu,
      lengths,
      lights_collections,
      lights_insert_order,
      mapping_length_idx,
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
    for lid in &self.lights.lights_insert_order {
      let lights = self.lights.lights_collections.get(lid).unwrap();
      lights.as_ref().as_ref().setup_pass(ctx)
    }
    self.lighting.tonemap.setup_pass(ctx);
  }
}

impl<'a> ShaderHashProvider for ForwardSceneLightingDispatcher<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.base.hash_pipeline(hasher);
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
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    self.hash_pipeline(hasher);
    // this is so special(I think) that id could skip
  }
}

impl<'a> GraphicsShaderProvider for ForwardSceneLightingDispatcher<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.base.build(builder)
  }
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
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

      let alpha = builder.query::<AlphaChannel>().unwrap_or_else(|_| val(1.0));

      // should we use other way to get mask mode?
      let alpha = if builder.query::<AlphaCutChannel>().is_ok() {
        if_by(alpha.equals(val(0.)), || builder.discard());
        val(1.)
      } else {
        alpha
      };

      builder.store_fragment_out(0, (ldr, alpha))
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
    builder: &mut ShaderRenderPipelineBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
  ) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let lengths_info = binding.bind_by(&self.lengths);
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

      let mut light_specular_result = val(Vec3::zero());
      let mut light_diffuse_result = val(Vec3::zero());

      for (i, lid) in self.lights_insert_order.iter().enumerate() {
        let lights = self.lights_collections.get(lid).unwrap();
        let length = lengths_info.index(val(i as u32)).load().x();
        builder.register::<LightCount>(length);

        let ENode::<ShaderLightingResult> { diffuse, specular } = lights
          .as_ref()
          .as_ref()
          .compute_lights(builder, binding, shading_impl, shading.as_ref(), &geom_ctx);
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
    Self: Stream<Item = Vec<(u64, Option<T>)>>;
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
    Self: Stream<Item = Vec<(u64, Option<TT>)>>,
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
    loop {
      // compare to the flatten, we poll the outside stream first
      if let Poll::Ready(v) = this.stream.as_mut().poll_next(cx) {
        if let Some(v) = v {
          this.next.set(Some(v));
        } else {
          return Poll::Ready(None);
        }
      } else if let Some(mut s) = this.next.as_mut().as_pin_mut() {
        if let Some(s) = s.as_mut().as_pin_mut() {
          if let Some(item) = ready!(s.poll_next(cx)) {
            return Poll::Ready(Some(Some(item)));
          } else {
            this.next.set(None);
          }
        } else {
          return Poll::Pending;
        }
      } else {
        return Poll::Pending;
      }
    }
  }
}

impl<T, S> Stream for ReactiveLightList<S, T>
where
  T: ShaderLight,
  S: Stream<Item = Vec<(u64, Option<T>)>>,
{
  type Item = usize;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(Some(updates)) = this.input.poll_next(cx) {
      for (light_id, light) in updates {
        this.list.update(light_id, light);
      }
      Poll::Ready(this.list.maintain().into())
    } else {
      Poll::Pending
    }
  }
}
