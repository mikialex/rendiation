use crate::*;

// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/sections/ssao.md

const MAX_SAMPLE: usize = 64;
const MAX_NOISE: usize = 64;

pub struct SSAO {
  parameters: UniformBufferDataView<SSAOParameter>,
  samples: UniformBufferDataView<Shader140Array<f32, MAX_SAMPLE>>,
  noises: UniformBufferDataView<Shader140Array<f32, MAX_NOISE>>,
}

impl SSAO {
  pub fn new(gpu: &GPU) -> Self {
    let parameters = create_uniform(SSAOParameter::default(), gpu);

    let samples = todo!();
    let noises = todo!();

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
  pub radius: f32,
  pub bias: f32,
  pub magnitude: f32,
  pub contrast: f32,
}

impl Default for SSAOParameter {
  fn default() -> Self {
    Self {
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
  parameter: &'a UniformBufferDataView<SSAOParameter>,
}

impl<'a> ShaderHashProvider for AOComputer<'a> {}
impl<'a> ShaderHashProviderAny for AOComputer<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    struct Mark;
    Mark.type_id().hash(hasher)
  }
}
impl<'a> ShaderPassBuilder for AOComputer<'a> {}
impl<'a> ShaderGraphProvider for AOComputer<'a> {}

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
          parameter: &self.parameters,
        }
        .draw_quad(),
      );

    // blur

    todo!()
  }
}
