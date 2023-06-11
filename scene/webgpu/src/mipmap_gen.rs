use crate::*;

pub struct MipMapTaskManager {
  pub generator: Mipmap2DGenerator,
  tasks: Vec<GPU2DTexture>,
}

impl MipMapTaskManager {
  pub fn request_mipmap_gen(&mut self, texture: &GPU2DTexture) {
    self.tasks.push(texture.clone())
  }

  pub fn cancel_mipmap_gen(&mut self, texture: &GPU2DTexture) {
    if let Some(i) = self.tasks.iter().position(|t| t.0.guid == texture.0.guid) {
      self.tasks.remove(i);
    }
  }

  pub fn flush_mipmap_gen_request(&mut self, ctx: &mut FrameCtx) {
    for tex in self.tasks.drain(..) {
      self.generator.generate(ctx, &tex)
    }
  }
}

impl Default for MipMapTaskManager {
  fn default() -> Self {
    Self {
      generator: Mipmap2DGenerator::new(DefaultMipmapReducer),
      tasks: Default::default(),
    }
  }
}

// https://github.com/BabylonJS/Babylon.js/blob/d25bc29091/packages/dev/core/src/Engines/WebGPU/webgpuTextureHelper.ts

/// Mipmap generation is not supported in webgpu api for now, at least in mvp as far as i known.
/// It's also useful to provide customizable reducer / gen method for proper usage.
pub struct Mipmap2DGenerator {
  pub reducer: Box<dyn Mipmap2dReducer>,
}

impl Mipmap2DGenerator {
  pub fn new(reducer: impl Mipmap2dReducer + 'static) -> Self {
    Self {
      reducer: Box::new(reducer),
    }
  }

  pub fn generate(&self, ctx: &mut FrameCtx, texture: &GPU2DTexture) {
    for write_level in 1..texture.desc.mip_level_count {
      let write_view: GPU2DTextureView = texture
        .create_view(webgpu::TextureViewDescriptor {
          base_mip_level: write_level,
          mip_level_count: Some(1),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let read_level = write_level - 1;
      let read_view = texture
        .create_view(webgpu::TextureViewDescriptor {
          base_mip_level: read_level,
          mip_level_count: Some(1),
          base_array_layer: 0,
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let task = Mipmap2DGeneratorTask {
        view: read_view,
        reducer: self.reducer.as_ref(),
      }
      .draw_quad();

      pass("mip-gen-2d")
        .with_color(write_view, load())
        .render(ctx)
        .by(task);
    }
  }

  /// It's useful to generate cube faces use same method like 2d.
  /// even it's not correct from perspective of spherical filtering.
  pub fn generate_cube_faces(&self, ctx: &mut FrameCtx, texture: &GPUCubeTexture) {
    for write_level in 1..texture.desc.mip_level_count {
      for face in 0..texture.desc.size.depth_or_array_layers {
        let write_view: GPU2DTextureView = texture
          .create_view(webgpu::TextureViewDescriptor {
            base_mip_level: write_level,
            mip_level_count: Some(1),
            base_array_layer: face,
            // it defaults to None which defaults to cube type
            dimension: Some(webgpu::TextureViewDimension::D2),
            ..Default::default()
          })
          .try_into()
          .unwrap();

        let read_level = write_level - 1;
        let read_view = texture
          .create_view(webgpu::TextureViewDescriptor {
            base_mip_level: read_level,
            mip_level_count: Some(1),
            base_array_layer: face,
            ..Default::default()
          })
          .try_into()
          .unwrap();

        let task = Mipmap2DGeneratorTask {
          view: read_view,
          reducer: self.reducer.as_ref(),
        }
        .draw_quad();

        pass("mip-gen-cube-face")
          .with_color(write_view, load())
          .render(ctx)
          .by(task);
      }
    }
  }
}

/// layer reduce logic, layer by layer.
/// input previous layer, generate next layer.
/// `current` is the layer's current writing pixel coordinate, range from 0. to 1.
pub trait Mipmap2dReducer: Send + Sync {
  fn reduce(
    &self,
    previous_level: Node<ShaderTexture2D>,
    sampler: Node<ShaderSampler>,
    current: Node<Vec2<f32>>,
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

struct DefaultMipmapReducer;

impl Mipmap2dReducer for DefaultMipmapReducer {
  #[rustfmt::skip]
  fn reduce(
    &self,
    previous_level: Node<ShaderTexture2D>,
    sampler: Node<ShaderSampler>,
    current: Node<Vec2<f32>>,
    texel_size: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let mut r = previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new( 0.5,  0.5)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(-0.5, -0.5)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new(-0.5,  0.5)), consts(0.));
    r        += previous_level.sample_level(sampler, current + texel_size * consts(Vec2::new( 0.5, -0.5)), consts(0.));
    r / consts(4.)
  }
}
struct Mipmap2DGeneratorTask<'a> {
  view: GPU2DTextureView,
  reducer: &'a dyn Mipmap2dReducer,
}

impl<'a> ShaderPassBuilder for Mipmap2DGeneratorTask<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.view, SB::Pass);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu(), SB::Pass);
  }
}

impl<'a> ShaderHashProvider for Mipmap2DGeneratorTask<'a> {}
impl<'a> ShaderHashProviderAny for Mipmap2DGeneratorTask<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Mark;
    Mark.type_id().hash(hasher)
  }
}

impl<'a> ShaderGraphProvider for Mipmap2DGeneratorTask<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let position = builder.query::<FragmentPosition>()?.xy();
      let buffer_size = builder.query::<RenderBufferSize>()?;
      let texel_size = builder.query::<TexelSize>()? * consts(0.5);
      let previous_level = binding.uniform_by(&self.view, SB::Pass);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Pass);

      let result = self
        .reducer
        .reduce(previous_level, sampler, position / buffer_size, texel_size);

      builder.set_fragment_out(0, result)
    })
  }
}
