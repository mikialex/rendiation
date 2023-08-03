use shadergraph::*;

use crate::*;

pub trait RenderComponent: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder {
  fn render(&self, ctx: &mut GPURenderPassCtx, com: DrawCommand) {
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

    ctx.pass.draw_by_command(com)
  }
}

impl<T> RenderComponent for T where
  T: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder
{
}

pub trait RenderComponentAny: RenderComponent + ShaderHashProviderAny {}
impl<T> RenderComponentAny for T where T: RenderComponent + ShaderHashProviderAny {}

impl<'a> ShaderHashProvider for &'a dyn RenderComponentAny {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline(hasher)
  }
}
impl<'a> ShaderHashProviderAny for &'a dyn RenderComponentAny {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline_and_with_type_id(hasher)
  }
}
impl<'a> ShaderPassBuilder for &'a dyn RenderComponentAny {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).post_setup_pass(ctx);
  }
}
impl<'a> GraphicsShaderProvider for &'a dyn RenderComponentAny {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    (*self).build(builder)
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    (*self).post_build(builder)
  }
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

impl<'a, 'b> GraphicsShaderProvider for RenderEmitter<'a, 'b> {
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

pub struct BindingController<'a, T> {
  inner: &'a T,
  target: usize,
}
pub trait BindingSlotAssign: Sized {
  fn assign_binding_index(&self, index: usize) -> BindingController<Self> {
    BindingController {
      inner: self,
      target: index,
    }
  }
}
impl<T> BindingSlotAssign for T {}

impl<'a, T: ShaderHashProvider> ShaderHashProvider for BindingController<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline(hasher)
  }
}
impl<'a, T: ShaderHashProviderAny> ShaderHashProviderAny for BindingController<'a, T> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline_and_with_type_id(hasher)
    // note, the binding info should hashed by binding grouper if necessary
  }
}
impl<'a, T: ShaderPassBuilder> ShaderPassBuilder for BindingController<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let before = ctx.binding.set_binding_slot(self.target);
    self.inner.setup_pass(ctx);
    ctx.binding.set_binding_slot(before);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let before = ctx.binding.set_binding_slot(self.target);
    self.inner.post_setup_pass(ctx);
    ctx.binding.set_binding_slot(before);
  }
}
impl<'a, T: GraphicsShaderProvider> GraphicsShaderProvider for BindingController<'a, T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let before = builder.set_binding_slot(self.target);
    let r = self.inner.build(builder);
    builder.set_binding_slot(before);
    r
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let before = builder.set_binding_slot(self.target);
    let r = self.inner.post_build(builder);
    builder.set_binding_slot(before);
    r
  }
}
