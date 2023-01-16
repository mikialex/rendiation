use shadergraph::*;

use crate::*;

pub trait RenderComponent: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {
  fn render(&self, ctx: &mut GPURenderPassCtx, emitter: &dyn DrawcallEmitter) {
    let mut hasher = PipelineHasher::default();
    self.hash_pipeline(&mut hasher);

    let pipeline = ctx
      .gpu
      .device
      .get_or_cache_create_render_pipeline(hasher, |device| {
        device
          .build_pipeline_by_shadergraph(self.build_self().unwrap())
          .unwrap()
      });

    ctx.binding.reset();
    ctx.reset_vertex_binding_index();

    self.setup_pass_self(ctx);

    ctx.pass.set_pipeline_owned(&pipeline);

    ctx
      .binding
      .setup_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline);

    emitter.draw(ctx);
  }
}

impl<T> RenderComponent for T where T: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {}

pub trait RenderComponentAny: RenderComponent + ShaderHashProviderAny {}
impl<T> RenderComponentAny for T where T: RenderComponent + ShaderHashProviderAny {}

pub trait DrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx);
}

pub struct RenderEmitter<'a, 'b> {
  contents: &'a [&'b dyn RenderComponentAny],
}

impl<'a, 'b> RenderEmitter<'a, 'b> {
  pub fn new(contents: &'a [&'b dyn RenderComponentAny]) -> Self {
    Self { contents }
  }
}

impl<'a, 'b> ShaderPassBuilder for RenderEmitter<'a, 'b> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.contents.iter().for_each(|c| c.setup_pass(ctx));
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self
      .contents
      .iter()
      .rev()
      .for_each(|c| c.post_setup_pass(ctx));
  }
}

impl<'a, 'b> ShaderHashProvider for RenderEmitter<'a, 'b> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .contents
      .iter()
      .for_each(|com| com.hash_pipeline_and_with_type_id(hasher))
  }
}

impl<'a, 'b> ShaderGraphProvider for RenderEmitter<'a, 'b> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for c in self.contents {
      c.build(builder)?;
    }
    Ok(())
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for c in self.contents.iter().rev() {
      c.post_build(builder)?;
    }
    Ok(())
  }
}
