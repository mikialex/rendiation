use rendiation_shader_api::*;
use rendiation_shader_backend_naga::ShaderAPINagaImpl;

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
          .build_pipeline_by_shader_api(
            self
              .build_self(&|stage| Box::new(ShaderAPINagaImpl::new(stage)))
              .unwrap(),
          )
          .unwrap()
      });

    ctx.binding.reset();
    ctx.reset_vertex_binding_index();

    self.setup_pass_self(ctx);

    ctx
      .binding
      .setup_render_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline);

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
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline_with_type_info(hasher)
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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (*self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (*self).post_build(builder)
  }
}
impl ShaderHashProvider for Box<dyn RenderComponentAny> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline(hasher)
  }
}
impl ShaderHashProviderAny for Box<dyn RenderComponentAny> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher)
  }
}
impl ShaderPassBuilder for Box<dyn RenderComponentAny> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).post_setup_pass(ctx);
  }
}
impl GraphicsShaderProvider for Box<dyn RenderComponentAny> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (**self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (**self).post_build(builder)
  }
}

pub struct RenderSlice<'a, T> {
  contents: &'a [T],
}

impl<'a, T> RenderSlice<'a, T> {
  pub fn new(contents: &'a [T]) -> Self {
    Self { contents }
  }
}

impl<'a, T: RenderComponentAny> ShaderPassBuilder for RenderSlice<'a, T> {
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

impl<'a, T: RenderComponentAny> ShaderHashProvider for RenderSlice<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .contents
      .iter()
      .for_each(|com| com.hash_pipeline_with_type_info(hasher))
  }
}

impl<'a, T: RenderComponentAny> GraphicsShaderProvider for RenderSlice<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    for c in self.contents {
      c.build(builder)?;
    }
    Ok(())
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    for c in self.contents.iter().rev() {
      c.post_build(builder)?;
    }
    Ok(())
  }
}

pub struct RenderArray<const N: usize, T> {
  pub contents: [T; N],
}

impl<const N: usize, T: RenderComponentAny> RenderArray<N, T> {
  pub fn as_slice(&self) -> impl RenderComponent + '_ {
    RenderSlice {
      contents: self.contents.as_slice(),
    }
  }
}

impl<const N: usize, T: RenderComponentAny> ShaderPassBuilder for RenderArray<N, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().setup_pass(ctx)
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().setup_pass(ctx)
  }
}

impl<const N: usize, T: RenderComponentAny> ShaderHashProvider for RenderArray<N, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.as_slice().hash_pipeline(hasher)
  }
}

impl<const N: usize, T: RenderComponentAny> GraphicsShaderProvider for RenderArray<N, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.as_slice().build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.as_slice().post_build(builder)
  }
}

pub struct BindingController<T> {
  inner: T,
  target: usize,
}
pub trait BindingSlotAssign: Sized {
  fn assign_binding_index(&self, index: usize) -> BindingController<&Self> {
    BindingController {
      inner: self,
      target: index,
    }
  }
  fn into_assign_binding_index(self, index: usize) -> BindingController<Self> {
    BindingController {
      inner: self,
      target: index,
    }
  }
}
impl<T> BindingSlotAssign for T {}

impl<T: ShaderHashProvider> ShaderHashProvider for BindingController<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline(hasher)
  }
}
impl<T: ShaderHashProviderAny> ShaderHashProviderAny for BindingController<T> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline_with_type_info(hasher)
    // note, the binding info should hashed by binding grouper if necessary
  }
}
impl<T: ShaderPassBuilder> ShaderPassBuilder for BindingController<T> {
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
impl<T: GraphicsShaderProvider> GraphicsShaderProvider for BindingController<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let before = builder.set_binding_slot(self.target);
    let r = self.inner.build(builder);
    builder.set_binding_slot(before);
    r
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let before = builder.set_binding_slot(self.target);
    let r = self.inner.post_build(builder);
    builder.set_binding_slot(before);
    r
  }
}
