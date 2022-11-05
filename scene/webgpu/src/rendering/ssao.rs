use crate::*;

// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/sections/ssao.md

const MAX_SAMPLE: usize = 64;
const MAX_NOISE: usize = 64;

pub struct SSAO {
  parameters: UniformBufferDataView<SSAOParameter>,
  samples: UniformBufferDataView<Shader140Array<Vec4<f32>, MAX_SAMPLE>>,
  noises: UniformBufferDataView<Shader140Array<Vec4<f32>, MAX_NOISE>>,
}

fn rand() -> f32 {
  rand::random()
}

impl SSAO {
  pub fn new(gpu: &GPU) -> Self {
    let parameters = SSAOParameter::default();

    // improve, try other low discrepancy serials
    let samples: Vec<Vec4<f32>> = (0..parameters.sample_count)
      .into_iter()
      .map(|i| {
        // generate point in half sphere
        let rand_p = loop {
          let rand_p = Vec3::new(rand() * 2. - 1., rand() * 2. - 1., rand());
          if rand_p.length() < 1. {
            break rand_p;
          }
        };
        let rand_p = Vec4::new(rand_p.x, rand_p.y, rand_p.z, 0.);
        let scale = (i as f32) / (parameters.sample_count as f32);
        rand_p * scale
      })
      .collect();
    let samples = samples.try_into().unwrap();
    let samples = create_uniform(samples, gpu);

    let noises: Vec<Vec4<f32>> = (0..parameters.sample_count)
      .into_iter()
      .map(|i| Vec4::new(rand() * 2. - 1., rand() * 2. - 1., rand() * 2. - 1., 0.))
      .collect();
    let noises = noises.try_into().unwrap();
    let noises = create_uniform(noises, gpu);

    let parameters = create_uniform(parameters, gpu);

    Self {
      parameters,
      samples,
      noises,
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
}

impl Default for SSAOParameter {
  fn default() -> Self {
    Self {
      noise_size: 16,
      sample_count: 16,
      radius: 0.6,
      bias: 0.005,
      magnitude: 1.1,
      contrast: 1.1,
      ..Zeroable::zeroed()
    }
  }
}

struct AOComputer<'a> {
  normal: AttachmentView<&'a Attachment>,
  depth: AttachmentView<&'a Attachment>,
  parameter: &'a SSAO,
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
    ctx.binding.bind(&self.normal, SB::Pass);
    ctx.binding.bind(&self.depth, SB::Pass);
    ctx.binding.bind(&self.parameter.parameters, SB::Pass);
    ctx.binding.bind(&self.parameter.samples, SB::Pass);
    ctx.binding.bind(&self.parameter.noises, SB::Pass);
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Material);
  }
}
impl<'a> ShaderGraphProvider for AOComputer<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let normal_tex = binding.uniform_by(&self.normal, SB::Pass);
      let depth_tex = binding.uniform_by(&self.depth, SB::Pass);
      let parameter = binding
        .uniform_by(&self.parameter.parameters, SB::Pass)
        .expand();
      let samples = binding.uniform_by(&self.parameter.samples, SB::Pass);
      let noises = binding.uniform_by(&self.parameter.noises, SB::Pass);
      let sampler = binding.uniform::<GPUSamplerView>(SB::Material);

      let uv = builder.query::<FragmentUv>()?;

      let iter = ClampedShaderIter {
        source: samples,
        count: parameter.sample_count,
      };

      let sample_count_f = parameter.sample_count.into_f32();

      let occlusion = sample_count_f.mutable();

      let depth = depth_tex.sample(sampler, uv).x();
      let position_world: Node<Vec3<f32>> = todo!();
      let normal = normal_tex.sample(sampler, uv).xyz();

      let noiseS = consts((MAX_NOISE as f32).sqrt() as u32);
      let noiseX = uv.x().into_u32() % noiseS;
      let noiseY = uv.y().into_u32() % noiseS;
      let random = noises.index(noiseX + (noiseY * noiseS)).xyz();

      let tangent = (random - normal * random.dot(normal)).normalize();
      let binormal = normal.cross(tangent);
      let tbn: Node<Mat3<f32>> = (tangent, binormal, normal).into();

      for_by(iter, |_, sample, index| {
        let sample_position_offset = tbn * sample.xyz();
        let sample_position_world = position_world + sample_position_offset * parameter.radius;

        let sample_position_ndc = sample_position_world; // todo
        let sample_position_depth = depth_tex.sample(sampler, sample_position_ndc.xy()).x();

        let occluded = (sample_position_depth + parameter.bias)
          .less_or_equal_than(sample_position_ndc.y())
          .select(consts(0.), consts(1.));

        let relative_depth_diff =
          parameter.radius / (sample_position_depth - sample_position_ndc.y()).abs();
        let intensity = relative_depth_diff.smoothstep(consts(0.), consts(1.));

        let occluded = occluded * intensity;
        occlusion.set(occlusion.get() - occluded);
      });

      let occlusion = occlusion.get() / sample_count_f;
      let occlusion = occlusion.pow(parameter.magnitude);
      let occlusion = parameter.contrast * (occlusion - consts(0.5)) + consts(0.5);

      builder.set_fragment_out(0, (occlusion.splat(), 1.))
    })
  }
}

impl SSAO {
  pub fn draw(&self, ctx: &mut FrameCtx, depth: &Attachment, normal: &Attachment) -> Attachment {
    let mut ao_result = attachment()
      .format(webgpu::TextureFormat::Rgba8Unorm) // todo half resolution?
      .request(ctx);

    pass("ssao-compute")
      .with_color(ao_result.write(), load())
      .render(ctx)
      .by(
        AOComputer {
          normal: normal.read(),
          depth: depth.read(),
          parameter: self,
        }
        .draw_quad(),
      );

    // blur

    ao_result
  }
}
