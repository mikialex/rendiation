// https://www.elopezr.com/temporal-aa-and-the-quest-for-the-holy-trail/#more-3285
// https://sugulee.wordpress.com/2021/06/21/temporal-anti-aliasingtaa-tutorial/

use rendiation_shader_library::{shader_uv_space_to_world_space, shader_world_space_to_uv_space};

use crate::*;

const SAMPLE_COUNT: usize = 32;

pub struct TAA {
  frame_index: usize,
  jitters: Vec<Vec2<f32>>,
  history: Option<Attachment>,
}

pub struct NewTAAFrameSample {
  pub new_color: Attachment,
  pub new_depth: Attachment,
}

pub trait TAAContent {
  fn set_jitter(&mut self, next_jitter: Vec2<f32>);
  // the reproject info maybe useful in
  fn render(self, ctx: &mut FrameCtx) -> NewTAAFrameSample;
}

impl TAA {
  pub fn new() -> Self {
    Self {
      frame_index: 0,
      jitters: (0..SAMPLE_COUNT).map(halton23).collect(),
      history: None,
    }
  }

  pub fn render_aa_content(
    &mut self,
    mut content: impl TAAContent,
    ctx: &mut FrameCtx,
    reproject: &GPUReprojectInfo,
  ) -> &Attachment {
    content.set_jitter(self.next_jitter());

    let NewTAAFrameSample {
      new_color,
      new_depth,
    } = content.render(ctx);

    self.resolve(&new_color, &new_depth, ctx, reproject)
  }

  fn next_jitter(&mut self) -> Vec2<f32> {
    let r = self.jitters[self.frame_index % SAMPLE_COUNT];
    self.frame_index += 1;
    r
  }

  fn resolve(
    &mut self,
    new_color: &Attachment,
    new_depth: &Attachment,
    ctx: &mut FrameCtx,
    reproject: &GPUReprojectInfo,
  ) -> &Attachment {
    let mut resolve_target = attachment()
      .format(TextureFormat::Rgba8UnormSrgb)
      .request(ctx);

    let history = self.history.get_or_insert_with(|| {
      attachment()
        .format(TextureFormat::Rgba8UnormSrgb)
        .request(ctx)
    });

    pass("taa-resolve")
      .with_color(resolve_target.write(), load())
      .render_ctx(ctx)
      .by(
        &mut TAAResolver {
          history: history.read(),
          new_color: new_color.read(),
          new_depth: new_depth.read(),
          reproject,
        }
        .draw_quad(),
      );

    // note, if the history size different than current, it's still works fine
    // and the history will be correct update to new size
    std::mem::swap(history, &mut resolve_target);

    history
  }
}

impl Default for TAA {
  fn default() -> Self {
    Self::new()
  }
}

struct TAAResolver<'a> {
  history: AttachmentView<&'a Attachment>,
  new_color: AttachmentView<&'a Attachment>,
  new_depth: AttachmentView<&'a Attachment>,
  reproject: &'a GPUReprojectInfo,
}

impl GraphicsShaderProvider for TAAResolver<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
      let color_sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let history = binding.bind_by(&self.history);
      let new = binding.bind_by(&self.new_color);
      let new_depth = binding.bind_by(&DisableFiltering(&self.new_depth));

      let reproject = binding.bind_by(&self.reproject.reproject).load().expand();

      let uv = builder.query::<FragmentUv>();

      let depth = new_depth.sample(sampler, uv).x();

      let world_position =
        shader_uv_space_to_world_space(reproject.current_camera_view_projection_inv, uv, depth);
      let (reproject_uv, _) =
        shader_world_space_to_uv_space(reproject.previous_camera_view_projection, world_position);

      let previous = history.sample(color_sampler, reproject_uv);

      let texel_size = builder.query::<TexelSize>();
      let previous_clamped = clamp_color(new, color_sampler, texel_size, uv, previous.xyz());

      let new = new.sample(color_sampler, uv).xyz();

      let ratio = 0.1;

      let output = new * val(ratio) + previous_clamped * val(1. - ratio);

      builder.store_fragment_out(0, (output, val(1.)))
    })
  }
}

fn clamp_color(
  tex: HandleNode<ShaderTexture2D>,
  sp: HandleNode<ShaderSampler>,
  texel_size: Node<Vec2<f32>>,
  position: Node<Vec2<f32>>,
  previous: Node<Vec3<f32>>,
) -> Node<Vec3<f32>> {
  let mut min_c = val(Vec3::one());
  let mut max_c = val(Vec3::zero());

  // unloop
  for i in -1..=1 {
    for j in -1..=1 {
      let offset = val::<Vec2<_>>((i as f32, j as f32).into());
      let sample = tex.sample(sp, position + texel_size * offset).xyz();
      min_c = min_c.min(sample);
      max_c = max_c.max(sample);
    }
  }

  previous.clamp(min_c, max_c)
}

impl ShaderPassBuilder for TAAResolver<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.bind_immediate_sampler(
      &TextureSampler {
        min_filter: rendiation_texture_core::FilterMode::Linear,
        mag_filter: rendiation_texture_core::FilterMode::Linear,
        ..Default::default()
      }
      .into_gpu(),
    );
    ctx.binding.bind(&self.history);
    ctx.binding.bind(&self.new_color);
    ctx.binding.bind(&self.new_depth);
    ctx.binding.bind(&self.reproject.reproject);
  }
}
impl ShaderHashProvider for TAAResolver<'_> {
  shader_hash_type_id! {TAAResolver<'static>}
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
