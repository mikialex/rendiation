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

impl<'a> ShaderHashProvider for &'a dyn RenderComponent {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline_with_type_info(hasher)
  }

  shader_hash_type_id! {&'static dyn RenderComponent}
}

impl<'a> ShaderPassBuilder for &'a dyn RenderComponent {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).post_setup_pass(ctx);
  }
}
impl<'a> GraphicsShaderProvider for &'a dyn RenderComponent {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (*self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (*self).post_build(builder)
  }
}
impl<'a> ShaderHashProvider for Box<dyn RenderComponent + 'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
  shader_hash_type_id! {Box<dyn RenderComponent>}
}
impl<'a> ShaderPassBuilder for Box<dyn RenderComponent + 'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).post_setup_pass(ctx);
  }
}
impl<'a> GraphicsShaderProvider for Box<dyn RenderComponent + 'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (**self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    (**self).post_build(builder)
  }
}

pub struct RenderSlice<'a, T>(pub &'a [T]);

impl<'a, T: RenderComponent> ShaderPassBuilder for RenderSlice<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.iter().for_each(|c| c.setup_pass(ctx));
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.iter().rev().for_each(|c| c.post_setup_pass(ctx));
  }
}

impl<'a, T: RenderComponent> ShaderHashProvider for RenderSlice<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .0
      .iter()
      .for_each(|com| com.hash_pipeline_with_type_info(hasher))
  }

  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<RenderSlice<'static, ()>>().hash(hasher);
    // is it ok??
    if let Some(com) = self.0.last() {
      com.hash_type_info(hasher);
    }
  }
}

impl<'a, T: RenderComponent> GraphicsShaderProvider for RenderSlice<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    for c in self.0 {
      c.build(builder)?;
    }
    Ok(())
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    for c in self.0.iter().rev() {
      c.post_build(builder)?;
    }
    Ok(())
  }
}

pub struct RenderArray<const N: usize, T>(pub [T; N]);

impl<const N: usize, T: RenderComponent> RenderArray<N, T> {
  pub fn as_slice(&self) -> impl RenderComponent + '_ {
    RenderSlice(self.0.as_slice())
  }
}

impl<const N: usize, T: RenderComponent> ShaderPassBuilder for RenderArray<N, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().setup_pass(ctx)
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().post_setup_pass(ctx)
  }
}

impl<const N: usize, T: RenderComponent> ShaderHashProvider for RenderArray<N, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.as_slice().hash_pipeline(hasher)
  }
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.as_slice().hash_type_info(hasher)
  }
}

impl<const N: usize, T: RenderComponent> GraphicsShaderProvider for RenderArray<N, T> {
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

  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    TypeId::of::<BindingController<()>>().hash(hasher);
    self.inner.hash_type_info(hasher)
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

/// weaker version of RenderComponent, only inject shader dependencies
pub trait RenderDependencyComponent:
  ShaderHashProvider + GraphicsShaderDependencyProvider + ShaderPassBuilder
{
}
impl<T> RenderDependencyComponent for T where
  T: ShaderHashProvider + GraphicsShaderDependencyProvider + ShaderPassBuilder
{
}
