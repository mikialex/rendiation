use crate::*;

pub struct MaterialDeferPassResult {
  world_position: Attachment,
  depth: Attachment,
  normal: Attachment,
  material: Attachment,
}

const WORLD_POSITION_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
const NORMAL_FORMAT: TextureFormat = TextureFormat::Rg32Float;
const MATERIAL_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

impl DeferGBufferSchema<PhysicalShading> for MaterialDeferPassResult {
  fn reconstruct_geometry_ctx(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<ShaderLightingGeometricCtx> {
    todo!()
  }

  fn reconstruct_shading(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<PhysicalShading> {
    todo!()
  }
}

pub struct GBufferEncodeTask {}

impl ShaderGraphProvider for GBufferEncodeTask {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(WORLD_POSITION_FORMAT));
      builder.define_out_by(channel(NORMAL_FORMAT));
      builder.define_out_by(channel(MATERIAL_FORMAT));
      Ok(())
    })
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      // collect dependency,
      let shading = PhysicalShading::construct_shading(builder);
      let world_position = builder.query::<FragmentWorldPosition>();
      let world_normal = builder.query::<FragmentWorldNormal>();
      // override channel writes
      todo!();
      Ok(()) //
    })
  }
}

impl MaterialDeferPassResult {
  pub fn new(ctx: &mut FrameCtx) -> Self {
    let world_position = attachment().format(WORLD_POSITION_FORMAT).request(ctx);
    let depth = depth_attachment().request(ctx);
    let normal = attachment().format(NORMAL_FORMAT).request(ctx);
    let material = attachment().format(MATERIAL_FORMAT).request(ctx);
    Self {
      world_position,
      depth,
      normal,
      material,
    }
  }
}

pub struct DeferLightingSystem {
  pub lights: Vec<Box<dyn Any>>,
}

pub fn defer(
  tonemap: &ToneMap,
  content: usize,
  ctx: &mut FrameCtx,
  lights: &DeferLightingSystem,
  shading: &impl LightableSurfaceShading,
) -> Attachment {
  // encode pass,
  let mut encode_target = MaterialDeferPassResult::new(ctx);

  {
    let encode_pass = pass("defer_encode_gbuffer")
      .with_depth(encode_target.depth.write(), clear(1.))
      .with_color(encode_target.world_position.write(), clear(all_zero()))
      .with_color(encode_target.normal.write(), clear(all_zero()))
      .with_color(encode_target.material.write(), clear(all_zero()))
      .render(ctx);
    // .by(todo!());
  }

  let mut hdr_result = attachment().format(TextureFormat::Rgba32Float).request(ctx);

  // light pass,
  for lights in &lights.lights {
    // let defer = DrawDefer {
    //   light: todo!(),
    //   defer: todo!(),
    //   shading,
    //   target: todo!(),
    // };

    pass("light_pass")
      .with_color(hdr_result.write(), load())
      .render(ctx);
    // lights.drasw_defer_passes(ctx)
  }

  // tone mapping,
  let mut ldr_result = attachment().format(TextureFormat::Rgba8Unorm).request(ctx);

  pass("tonemap")
    .with_color(ldr_result.write(), load())
    .render(ctx)
    .by(tonemap.tonemap(hdr_result.read()));

  ldr_result
}

pub trait ShaderLightPassApply {
  fn draw_defer_impl(active_pass: &mut ActiveRenderPass, shading: &impl LightableSurfaceShading);
}

impl<T: ShaderLight> ShaderLightPassApply for T {
  fn draw_defer_impl(active_pass: &mut ActiveRenderPass, shading: &impl LightableSurfaceShading) {
    todo!()
  }
}

/// define a specific g buffer layout.
///
/// this trait is parameterized over shading, which means we could encode/reconstruct
/// different surface shading into one schema theoretically
pub trait DeferGBufferSchema<S: LightableSurfaceShading> {
  fn reconstruct_geometry_ctx(
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> ExpandedNode<ShaderLightingGeometricCtx>;

  fn reconstruct_shading(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<S>;
}

/// define a specific light buffer layout.
pub trait LightBufferSchema {
  fn write_lighting(
    builder: &mut ShaderGraphFragmentBuilder,
    result: ExpandedNode<ShaderLightingResult>,
  );
}

pub struct DrawDefer<'a, D, S, R> {
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
      let geom_ctx = D::reconstruct_geometry_ctx(builder);

      let shading = D::reconstruct_shading(builder);

      let result =
        self
          .light
          .compute_lights_grouped(builder, binding, self.shading, &shading, &geom_ctx)?;

      R::write_lighting(builder, result);

      Ok(())
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
  }
}
