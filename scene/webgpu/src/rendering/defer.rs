use crate::*;

pub struct MaterialDeferPassResult {
  world_position: Attachment,
  depth: Attachment,
  normal: Attachment,
  // todo, merge material2 to normal, use ycocog encode for specular3->2
  material1: Attachment, // diffuse3+roughness1
  material2: Attachment, // specular3
}

const WORLD_POSITION_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
const NORMAL_FORMAT: TextureFormat = TextureFormat::Rg32Float;
const MATERIAL1_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const MATERIAL2_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

impl DeferGBufferSchema<PhysicalShading> for MaterialDeferPassResult {
  fn reconstruct(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
  ) -> Result<
    (
      ExpandedNode<ShaderLightingGeometricCtx>,
      ExpandedNode<ShaderPhysicalShading>,
    ),
    ShaderGraphBuildError,
  > {
    let world_position = binding.uniform_by(&self.world_position.read(), SB::Pass);
    let normal = binding.uniform_by(&self.normal.read(), SB::Pass);
    let material1 = binding.uniform_by(&self.material1.read(), SB::Pass);
    let material2 = binding.uniform_by(&self.material2.read(), SB::Pass);

    let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

    let uv = builder.query::<FragmentUv>()?;

    let world_position = world_position.sample(sampler, uv).xyz();
    let normal = normal.sample(sampler, uv).xyz();
    let material1 = material1.sample(sampler, uv);
    let material2 = material2.sample(sampler, uv);

    let camera_position = builder.query::<CameraWorldMatrix>()?.position();

    let geom_ctx = ExpandedNode::<ShaderLightingGeometricCtx> {
      position: world_position,
      normal,
      view_dir: camera_position - world_position,
    };

    let shading = ExpandedNode::<ShaderPhysicalShading> {
      diffuse: material1.xyz(),
      specular: material2.xyz(),
      roughness: material1.w(),
    };

    Ok((geom_ctx, shading))
  }
}

impl ShaderPassBuilder for MaterialDeferPassResult {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.world_position.read(), SB::Pass);
    ctx.binding.bind(&self.depth.read(), SB::Pass);
    ctx.binding.bind(&self.normal.read(), SB::Pass);
    ctx.binding.bind(&self.material1.read(), SB::Pass);
    ctx.binding.bind(&self.material2.read(), SB::Pass);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
  }
}

pub struct GBufferEncodeTask<T> {
  objects: T,
}

impl<'i, T> PassContentWithCamera for GBufferEncodeTask<T>
where
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    for model in self.objects {
      model.render(pass, &GBufferEncodeTaskDispatcher {}, camera)
    }
  }
}

struct GBufferEncodeTaskDispatcher {}
impl DispatcherDynSelf for GBufferEncodeTaskDispatcher {}
impl ShaderHashProvider for GBufferEncodeTaskDispatcher {}
impl ShaderPassBuilder for GBufferEncodeTaskDispatcher {}
impl ShaderGraphProvider for GBufferEncodeTaskDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(WORLD_POSITION_FORMAT));
      builder.define_out_by(channel(NORMAL_FORMAT));
      builder.define_out_by(channel(MATERIAL1_FORMAT));
      Ok(())
    })
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      // collect dependency
      let shading = PhysicalShading::construct_shading(builder);
      let world_position = builder.query::<FragmentWorldPosition>()?;
      let world_normal = builder.query::<FragmentWorldNormal>()?;
      // override channel writes
      builder.set_fragment_out(0, (world_position, 1.))?;
      builder.set_fragment_out(1, (world_normal, 1.))?;
      builder.set_fragment_out(2, (shading.diffuse, shading.roughness))?;
      builder.set_fragment_out(3, (shading.specular, 1.))?;
      Ok(())
    })
  }
}

pub struct DeferLightingSystem {
  pub lights: Vec<Box<dyn VisitLightCollectionCompute>>,
}

pub fn defer<'i, T>(
  tonemap: &ToneMap,
  objects: T,
  ctx: &mut FrameCtx,
  lights: &DeferLightingSystem,
  camera: &SceneCamera,
) -> Attachment
where
  T: IntoIterator<Item = &'i dyn SceneRenderable> + Copy,
{
  let mut encode_target = MaterialDeferPassResult {
    world_position: attachment().format(WORLD_POSITION_FORMAT).request(ctx),
    depth: depth_attachment().request(ctx),
    normal: attachment().format(NORMAL_FORMAT).request(ctx),
    material1: attachment().format(MATERIAL1_FORMAT).request(ctx),
    material2: attachment().format(MATERIAL2_FORMAT).request(ctx),
  };

  pass("defer_encode_gbuffer")
    .with_depth(encode_target.depth.write(), clear(1.))
    .with_color(encode_target.world_position.write(), clear(all_zero()))
    .with_color(encode_target.normal.write(), clear(all_zero()))
    .with_color(encode_target.material1.write(), clear(all_zero()))
    .render(ctx)
    .by(CameraRef::with(camera, GBufferEncodeTask { objects }));

  let mut hdr_result = attachment().format(TextureFormat::Rgba32Float).request(ctx);

  for lights in &lights.lights {
    lights.visit_lights_computes(&mut |light| {
      let defer = DrawDefer {
        light,
        defer: &encode_target,
        shading: &PhysicalShading,
        target: &SimpleLightSchema,
      }
      .draw_quad();

      pass("light_pass")
        .with_color(hdr_result.write(), load())
        .render(ctx)
        .by(defer);
    });
  }

  let mut ldr_result = attachment().format(TextureFormat::Rgba8Unorm).request(ctx);

  pass("tonemap")
    .with_color(ldr_result.write(), load())
    .render(ctx)
    .by(tonemap.tonemap(hdr_result.read()));

  ldr_result
}

pub trait VisitLightCollectionCompute {
  fn visit_lights_computes(&self, visitor: &mut dyn FnMut(&dyn LightCollectionCompute));
}

pub struct DeferLightList<T: ShaderLight> {
  pub lights: Vec<T>,
  pub lights_gpu: Vec<UniformBufferDataView<T>>,
}

impl<T: ShaderLight> VisitLightCollectionCompute for DeferLightList<T> {
  fn visit_lights_computes(&self, visitor: &mut dyn FnMut(&dyn LightCollectionCompute)) {
    self
      .lights_gpu
      .iter()
      .for_each(|light| visitor(&SingleLight { light }))
  }
}

struct SingleLight<'a, T: Std140> {
  light: &'a UniformBufferDataView<T>,
}

impl<'a, T: Std140> ShaderPassBuilder for SingleLight<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.light, SB::Pass)
  }
}
impl<'a, T: Std140> ShaderHashProvider for SingleLight<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<T>().hash(hasher)
  }
}
impl<'a, T: ShaderLight> LightCollectionCompute for SingleLight<'a, T> {
  fn compute_lights(
    &self,
    builder: &mut ShaderGraphFragmentBuilderView,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    shading_impl: &dyn LightableSurfaceShadingDyn,
    shading: &dyn Any,
    geom_ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> Result<(Node<Vec3<f32>>, Node<Vec3<f32>>), ShaderGraphBuildError> {
    let light = binding.uniform_by(self.light, SB::Pass);

    let dep = T::create_dep(builder);

    let light = light.expand();
    let incident = T::compute_direct_light(&light, &dep, geom_ctx);
    let light_result = shading_impl.compute_lighting_dyn(shading, &incident, geom_ctx);

    Ok((light_result.diffuse, light_result.specular))
  }
}

/// define a specific g buffer layout.
///
/// this trait is parameterized over shading, which means we could encode/reconstruct
/// different surface shading into one schema theoretically
pub trait DeferGBufferSchema<S: LightableSurfaceShading> {
  fn reconstruct(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
  ) -> Result<
    (
      ExpandedNode<ShaderLightingGeometricCtx>,
      ExpandedNode<S::ShaderStruct>,
    ),
    ShaderGraphBuildError,
  >;
}

/// define a specific light buffer layout.
pub trait LightBufferSchema {
  fn write_lighting(
    builder: &mut ShaderGraphFragmentBuilder,
    result: ExpandedNode<ShaderLightingResult>,
  ) -> Result<(), ShaderGraphBuildError>;
}

pub struct SimpleLightSchema;
impl LightBufferSchema for SimpleLightSchema {
  fn write_lighting(
    builder: &mut ShaderGraphFragmentBuilder,
    result: ExpandedNode<ShaderLightingResult>,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.set_fragment_out(0, ((result.specular + result.diffuse), 1.0))
  }
}

pub struct DrawDefer<'a, D, S, R> {
  /// this trait allow us using forward light list do batch light computation in single pass
  pub light: &'a dyn LightCollectionCompute,
  pub shading: &'a S,
  pub defer: &'a D,
  pub target: &'a R,
}

impl<'a, S, D, R> ShaderGraphProvider for DrawDefer<'a, D, S, R>
where
  S: LightableSurfaceShading,
  D: DeferGBufferSchema<S>,
  R: LightBufferSchema,
{
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let (geom_ctx, shading) = self.defer.reconstruct(builder, binding)?;

      let result =
        self
          .light
          .compute_lights_grouped(builder, binding, self.shading, &shading, &geom_ctx)?;

      R::write_lighting(builder, result)
    })
  }
}

impl<'a, D, S, R> ShaderHashProvider for DrawDefer<'a, D, S, R> {
  fn hash_pipeline(&self, _: &mut PipelineHasher) {}
}

impl<'a, D: Any, S: Any, R: Any> ShaderHashProviderAny for DrawDefer<'a, D, S, R> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<D>().hash(hasher);
    TypeId::of::<S>().hash(hasher);
    TypeId::of::<R>().hash(hasher);
    self.light.hash_pipeline(hasher);
  }
}

impl<'a, D: ShaderPassBuilder, S, R> ShaderPassBuilder for DrawDefer<'a, D, S, R> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.defer.setup_pass(ctx);
    self.light.setup_pass(ctx)
  }
}
