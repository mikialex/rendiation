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
      .map(|_| Vec4::new(rand() * 2. - 1., rand() * 2. - 1., rand() * 2. - 1., 0.))
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
  source_camera: &'a SceneCamera,
  /// this has to be cloned, because it's simple and easy
  source_camera_gpu: Option<UniformBufferDataView<CameraGPUTransform>>,
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
    ctx.bind_immediate_sampler(&TextureSampler::default(), SB::Pass);
    ctx
      .binding
      .bind(self.source_camera_gpu.as_ref().unwrap(), SB::Pass);
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
      let sampler = binding.uniform::<GPUSamplerView>(SB::Pass);

      let camera = binding
        .uniform_by(self.source_camera_gpu.as_ref().unwrap(), SB::Pass)
        .expand();

      let uv = builder.query::<FragmentUv>()?;

      let iter = ClampedShaderIter {
        source: samples,
        count: parameter.sample_count,
      };

      let sample_count_f = parameter.sample_count.into_f32();

      let occlusion = sample_count_f.mutable();

      let depth = depth_tex.sample(sampler, uv).x();
      let position_world = shader_uv_space_to_world_space(&camera, uv, depth);
      let normal = normal_tex.sample(sampler, uv).xyz();

      let noise_s = consts((MAX_NOISE as f32).sqrt() as u32);
      let noise_x = uv.x().into_u32() % noise_s;
      let noise_y = uv.y().into_u32() % noise_s;
      let random = noises.index(noise_x + (noise_y * noise_s)).xyz();

      let tangent = (random - normal * random.dot(normal)).normalize();
      let binormal = normal.cross(tangent);
      let tbn: Node<Mat3<f32>> = (tangent, binormal, normal).into();

      for_by(iter, |_, sample, _| {
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

// a little hack to get camera gpu without copy
impl<'a> PassContent for QuadDraw<AOComputer<'a>> {
  fn render(&mut self, pass: &mut SceneRenderPass) {
    let mut base = pass.default_dispatcher();
    let source_camera_gpu = &pass
      .resources
      .cameras
      .check_update_gpu(self.content.source_camera, pass.ctx.gpu)
      .ubo;

    self.content.source_camera_gpu = source_camera_gpu.clone().into();

    base.auto_write = false;
    let components: [&dyn RenderComponentAny; 3] = [&base, &self.quad, &self.content];
    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &self.quad);
  }
}

impl SSAO {
  pub fn draw(
    &self,
    ctx: &mut FrameCtx,
    depth: &Attachment,
    normal: &Attachment,
    source_camera: &SceneCamera,
  ) -> Attachment {
    let mut ao_result = attachment()
      .sizer(ratio_sizer(0.5)) // half resolution!
      .format(webgpu::TextureFormat::Rgba8Unorm)
      .request(ctx);

    pass("ssao-compute")
      .with_color(ao_result.write(), load())
      .render(ctx)
      .by(
        AOComputer {
          source_camera,
          source_camera_gpu: None,
          normal: normal.read(),
          depth: depth.read(),
          parameter: self,
        }
        .draw_quad(),
      );

    // todo blur

    ao_result
  }
}
