// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail/#more-3285
// https://sugulee.wordpress.com/2021/06/21/temporal-anti-aliasingtaa-tutorial/

use crate::*;

const SAMPLE_COUNT: usize = 32;

pub struct TAA {
  frame_index: usize,
  jitters: Vec<Vec2<f32>>,
  history: Option<Attachment>,
  current_camera: CameraGPU,
  previous_camera: CameraGPU,
}

impl TAA {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      frame_index: 0,
      jitters: (0..SAMPLE_COUNT).map(halton23).collect(),
      history: None,
      current_camera: CameraGPU::new(&gpu.device),
      previous_camera: CameraGPU::new(&gpu.device),
    }
  }

  pub fn next_jitter(&mut self) -> Vec2<f32> {
    let r = self.jitters[self.frame_index % SAMPLE_COUNT];
    self.frame_index += 1;
    r
  }

  pub fn resolve(
    &mut self,
    new_color: &Attachment,
    new_depth: &Attachment,
    ctx: &mut FrameCtx,
    new_camera: &CameraGPU,
  ) -> &Attachment {
    // improve? i think we could try copy buffer to buffer here.
    self
      .previous_camera
      .ubo
      .copy_cpu(&self.current_camera.ubo)
      .upload(&ctx.gpu.queue);

    self
      .current_camera
      .ubo
      .copy_cpu(&new_camera.ubo)
      .upload(&ctx.gpu.queue);

    let mut resolve_target = attachment()
      .format(webgpu::TextureFormat::Rgba8UnormSrgb)
      .request(ctx);

    let history = self.history.get_or_insert_with(|| {
      attachment()
        .format(webgpu::TextureFormat::Rgba8UnormSrgb)
        .request(ctx)
    });

    pass("taa-resolve")
      .with_color(resolve_target.write(), load())
      .render(ctx)
      .by(
        TAAResolver {
          history: history.read(),
          new_color: new_color.read(),
          new_depth: new_depth.read(),
          current_camera: &self.current_camera,
          previous_camera: &self.previous_camera,
        }
        .draw_quad(),
      );

    // note, if the history size different than current, it's still works fine
    // and the history will be correct update to new size
    std::mem::swap(history, &mut resolve_target);

    history
  }
}

struct TAAResolver<'a> {
  history: AttachmentView<&'a Attachment>,
  new_color: AttachmentView<&'a Attachment>,
  new_depth: AttachmentView<&'a Attachment>,
  current_camera: &'a CameraGPU,
  previous_camera: &'a CameraGPU,
}

impl<'a> GraphicsShaderProvider for TAAResolver<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let sampler = binding.binding::<DisableFiltering<GPUSamplerView>>();
      let color_sampler = binding.binding::<GPUSamplerView>();
      let history = binding.bind_by(&self.history);
      let new = binding.bind_by(&self.new_color);
      let new_depth = binding.bind_by(&DisableFiltering(&self.new_depth));

      let current_camera = binding.bind_by(&self.current_camera.ubo).expand();

      let previous_camera = binding.bind_by(&self.previous_camera.ubo).expand();

      let uv = builder.query::<FragmentUv>()?;

      let depth = new_depth.sample(sampler, uv).x();

      let world_position = shader_uv_space_to_world_space(&current_camera, uv, depth);
      let (reproject_uv, _) = shader_world_space_to_uv_space(&previous_camera, world_position);

      let previous = history.sample(color_sampler, reproject_uv);

      let texel_size = builder.query::<TexelSize>()?;
      let previous_clamped = clamp_color(new, color_sampler, texel_size, uv, previous.xyz());

      let new = new.sample(color_sampler, uv).xyz();

      let ratio = 0.1;

      let output = new * val(ratio) + previous_clamped * val(1. - ratio);

      builder.set_fragment_out(0, (output, 1.))
    })
  }
}

wgsl_fn!(
  fn clamp_color(
    tex: texture_2d<f32>,
    sp: sampler,
    texel_size: vec2<f32>,
    position: vec2<f32>,
    previous: vec3<f32>,
  ) -> vec3<f32> {
    var minC = vec3<f32>(1.);
    var maxC = vec3<f32>(0.);

    for(var i: i32 = -1; i <= 1; i++) {
      for(var j: i32 = -1; j <= 1; j++) {
        var sample = textureSample(tex, sp, position + vec2<f32>(f32(i),f32(j)) * texel_size).xyz;
        minC = min(minC, sample); maxC = max(maxC, sample);
      }
    }

    return clamp(previous, minC, maxC);
  }
);

impl<'a> ShaderPassBuilder for TAAResolver<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.bind_immediate_sampler(
      &TextureSampler {
        min_filter: rendiation_texture::FilterMode::Linear,
        mag_filter: rendiation_texture::FilterMode::Linear,
        ..Default::default()
      }
      .into_gpu(),
    );
    ctx.binding.bind(&self.history);
    ctx.binding.bind(&self.new_color);
    ctx.binding.bind(&self.new_depth);
    ctx.binding.bind(&self.current_camera.ubo);
    ctx.binding.bind(&self.previous_camera.ubo);
  }
}
impl<'a> ShaderHashProvider for TAAResolver<'a> {}
impl<'a> ShaderHashProviderAny for TAAResolver<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher)
  }
}

fn halton(index: usize, base: usize) -> f32 {
  let mut f = 1.;
  let mut r = 0.;
  let mut current = index;

  loop {
    f /= base as f32;
    r += f * (current % base) as f32;
    current = (current as f32 / base as f32).floor() as usize;
    if current == 0 {
      break;
    }
  }

  r
}

fn halton23(index: usize) -> Vec2<f32> {
  Vec2::new(halton(index + 1, 2), halton(index + 1, 3)) - Vec2::one()
}
