use crate::*;

// https://github.com/lettier/3d-game-shaders-for-beginners/blob/master/sections/ssao.md

pub struct SSAO {
  //
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
struct SSAOParameter {
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
  parameter: &'a UniformBufferData<SSAOParameter>,
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

pub fn ssao(ctx: &mut FrameCtx, depth: &Attachment, normal: &Attachment) -> Attachment {
  let ao_result = attachment()
    .format(webgpu::TextureFormat::Rgba8Unorm) // todo half resolution?
    .request(ctx);

  pass("ssao-compute")
    .with_color(ao_result.read(), load())
    .render(ctx)
    .by(
      AOComputer {
        normal: todo!(),
        depth: depth.read(),
        parameter: todo!(),
      }
      .draw_quad(),
    );

  // blur

  todo!()
}
