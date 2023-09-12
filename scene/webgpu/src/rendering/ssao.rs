use rendiation_shader_library::sampling::*;

use crate::*;

// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/sections/ssao.md

const MAX_SAMPLE: usize = 64;

pub struct SSAO {
  parameters: UniformBufferDataView<SSAOParameter>,
  samples: UniformBufferDataView<Shader140Array<Vec4<f32>, MAX_SAMPLE>>,
}

fn rand() -> f32 {
  rand::random()
}

impl SSAO {
  pub fn new(gpu: &GPU) -> Self {
    let parameters = SSAOParameter::default();

    // improve, try other low discrepancy serials
    let samples: Vec<Vec4<f32>> = (0..MAX_SAMPLE)
      .map(|i| {
        // generate point in half sphere
        let rand_p = loop {
          let rand_p = Vec3::new(rand() * 2. - 1., rand() * 2. - 1., rand());
          if rand_p.length() < 1. {
            break rand_p;
          }
        };
        let rand_p = rand_p.expand_with_one();
        let scale = (i as f32) / (parameters.sample_count as f32);
        rand_p * scale
      })
      .collect();
    let samples = samples.try_into().unwrap();
    let samples = create_uniform(samples, gpu);

    let parameters = create_uniform(parameters, gpu);

    Self {
      parameters,
      samples,
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct SSAOParameter {
  pub noise_size: u32,
  pub sample_count: u32,
  pub radius: f32,
  pub bias: f32,
  pub magnitude: f32,
  pub contrast: f32,
  pub noise_jit: f32,
}

impl Default for SSAOParameter {
  fn default() -> Self {
    Self {
      noise_size: 64,
      sample_count: 32,
      radius: 2.,
      bias: 0.0001,
      magnitude: 1.0,
      contrast: 1.5,
      noise_jit: 0.,
      ..Zeroable::zeroed()
    }
  }
}

pub struct AOComputer<'a> {
  depth: AttachmentView<&'a Attachment>,
  parameter: &'a SSAO,
  source_camera_gpu: &'a UniformBufferDataView<CameraGPUTransform>,
}

impl<'a> ShaderHashProvider for AOComputer<'a> {}
impl<'a> ShaderHashProviderAny for AOComputer<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Mark;
    Mark.type_id().hash(hasher)
  }
}
impl<'a> ShaderPassBuilder for AOComputer<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.depth);
    ctx.binding.bind(&self.parameter.parameters);
    ctx.binding.bind(&self.parameter.samples);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(self.source_camera_gpu);
  }
}
impl<'a> GraphicsShaderProvider for AOComputer<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let depth_tex = binding.bind_by(&DisableFiltering(&self.depth));
      let parameter = binding.bind_by(&self.parameter.parameters).load().expand();
      let samples = binding.bind_by(&self.parameter.samples);
      let sampler = binding.binding::<DisableFiltering<GPUSamplerView>>();

      let camera = binding.bind_by(self.source_camera_gpu).load().expand();

      let uv = builder.query::<FragmentUv>()?;

      let sample_count_f = parameter.sample_count.into_f32();

      let depth = depth_tex.sample(sampler, uv).x();
      let position_world = shader_uv_space_to_world_space(&camera, uv, depth);

      let normal = compute_normal_by_dxdy(position_world); // wrong, but i do not want pay cost to use normal texture input

      let random = random3_fn(uv + parameter.noise_jit.splat()) * val(2.) - val(Vec3::one());
      let tangent = (random - normal * random.dot(normal)).normalize();
      let binormal = normal.cross(tangent);
      let tbn = mat3_node((tangent, binormal, normal));

      let occlusion_sum = samples
        .into_shader_iter()
        .clamp_by(parameter.sample_count)
        .map(|(_, sample): (_, UniformNode<Vec4<f32>>)| {
          let sample_position_offset = tbn * sample.load().xyz();
          let sample_position_world = position_world + sample_position_offset * parameter.radius;

          let (s_uv, s_depth) = shader_world_space_to_uv_space(&camera, sample_position_world);
          // I think the naga's shader uniformity analysis is bugged if we use sample call here.
          let sample_position_depth = depth_tex.sample_level(sampler, s_uv, val(0.)).x();

          let occluded = (sample_position_depth + parameter.bias)
            .less_equal_than(s_depth)
            .select(0., 1.);

          let relative_depth_diff = parameter.radius / (sample_position_depth - s_depth).abs();
          let intensity = relative_depth_diff.smoothstep(val(0.), val(1.));

          occluded * intensity
        })
        .sum();

      let occlusion = parameter.sample_count.into_f32() - occlusion_sum;
      let occlusion = occlusion / sample_count_f;
      let occlusion = occlusion.pow(parameter.magnitude);
      let occlusion = parameter.contrast * (occlusion - val(0.5)) + val(0.5);

      builder.store_fragment_out(0, ((val(1.) - occlusion.saturate()).splat(), val(1.)))
    })
  }
}

// a little hack to get camera gpu without copy
impl<'a> PassContent for QuadDraw<AOComputer<'a>> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut base = default_dispatcher(pass);

    base.auto_write = false;
    let components: [&dyn RenderComponentAny; 3] = [&base, &self.quad, &self.content];
    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, QUAD_DRAW_CMD);
  }
}

impl SSAO {
  pub fn draw(
    &self,
    ctx: &mut FrameCtx,
    depth: &Attachment,
    source_camera_gpu: &CameraGPU,
  ) -> Attachment {
    self.parameters.mutate(|p| p.noise_jit = rand());
    self.parameters.upload(&ctx.gpu.queue);

    let mut ao_result = attachment()
      .sizer(ratio_sizer(0.5)) // half resolution!
      .format(webgpu::TextureFormat::Rgba8Unorm) // todo single channel
      .request(ctx);

    pass("ssao-compute")
      .with_color(ao_result.write(), load())
      .render(ctx)
      .by(
        AOComputer {
          source_camera_gpu: &source_camera_gpu.ubo,
          depth: depth.read(),
          parameter: self,
        }
        .draw_quad(),
      );

    // todo blur

    ao_result
  }
}
