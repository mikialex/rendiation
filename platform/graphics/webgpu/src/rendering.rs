use rendiation_shader_api::*;
use rendiation_shader_backend_naga::ShaderAPINagaImpl;

use crate::*;

pub enum RenderMethod<'a> {
  TraditionalDraw(DrawCommand),
  MeshPipelineDraw(&'a dyn MeshComponent),
}

pub enum MeshDispatchCommand {
  Direct(DispatchIndirectArgs),
  Indirect(GPUBufferResourceView),
}

pub trait MeshComponent: MeshShaderLogic + ShaderHashProvider {
  fn bind_shader(&self, cx: &mut GPURenderPassCtx);
  fn dispatch_command(&self) -> MeshDispatchCommand;
  fn as_shading_logic(&self) -> &dyn MeshShaderLogic;
}

/// RenderComponent is a composable unit for user to express and compose the rendering logic.
pub trait RenderComponent: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder {
  /// Calling this method to do the real drawcall on given pass. if the implementation is efficient enough to specify a draw logic.
  fn render(&self, ctx: &mut GPURenderPassCtx, draw: RenderMethod) {
    let mut hasher = PipelineHasher::default();
    self.hash_pipeline_with_type_info(&mut hasher);

    let (draw_cmd, mesh_shading) = match &draw {
      RenderMethod::TraditionalDraw(draw_command) => (Some(draw_command), None),
      RenderMethod::MeshPipelineDraw(mesh_shading_logic) => {
        mesh_shading_logic.hash_pipeline_with_type_info(&mut hasher);
        (None, Some(mesh_shading_logic.as_shading_logic()))
      }
    };

    let pipeline = ctx
      .gpu
      .device
      .get_or_cache_create_render_pipeline(hasher, |device, label| {
        device
          .build_pipeline_by_shader_api(
            self
              .build_self(
                &|stage| Box::new(ShaderAPINagaImpl::new(stage)),
                mesh_shading,
                ctx.gpu.info.clone(),
                device.inner.default_shader_checks,
              )
              .unwrap(),
            label,
          )
          .unwrap()
      });

    ctx.binding.reset();
    ctx.reset_vertex_binding_index();

    if ctx.enable_bind_check {
      ctx.binding.setup_checking_layout(&pipeline.bg_layouts);
    }

    if let RenderMethod::MeshPipelineDraw(draw) = draw {
      draw.bind_shader(ctx);
    }

    self.setup_pass_self(ctx);

    ctx
      .binding
      .setup_render_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline);

    if let Some(draw_cmd) = draw_cmd {
      ctx.pass.draw_by_command(draw_cmd.clone())
    } else if let RenderMethod::MeshPipelineDraw(draw) = draw {
      ctx.pass.dispatch_mesh_draw_command(draw.dispatch_command())
    }
  }
}

impl<T> RenderComponent for T where
  T: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder
{
}

impl ShaderHashProvider for &dyn RenderComponent {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (*self).hash_pipeline_with_type_info(hasher)
  }

  shader_hash_type_id! {&'static dyn RenderComponent}
}

impl ShaderPassBuilder for &dyn RenderComponent {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (*self).post_setup_pass(ctx);
  }
}
impl GraphicsShaderProvider for &dyn RenderComponent {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (*self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (*self).post_build(builder)
  }
}
impl ShaderHashProvider for Box<dyn RenderComponent + '_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    (**self).hash_pipeline_with_type_info(hasher);
  }
  shader_hash_type_id! {Box<dyn RenderComponent>}
}
impl ShaderPassBuilder for Box<dyn RenderComponent + '_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    (**self).post_setup_pass(ctx);
  }
}
impl GraphicsShaderProvider for Box<dyn RenderComponent + '_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (**self).build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    (**self).post_build(builder)
  }
}

pub struct RenderSlice<'a, T>(pub &'a [T]);

impl<T: RenderComponent> ShaderPassBuilder for RenderSlice<'_, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.iter().for_each(|c| c.setup_pass(ctx));
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.iter().rev().for_each(|c| c.post_setup_pass(ctx));
  }
}

impl<T: RenderComponent> ShaderHashProvider for RenderSlice<'_, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .0
      .iter()
      .for_each(|com| com.hash_pipeline_with_type_info(hasher))
  }

  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    hasher.hash_type::<RenderSlice<'static, ()>>();
    // is it ok??
    if let Some(com) = self.0.last() {
      com.hash_type_info(hasher);
    }
  }
}

impl<T: RenderComponent> GraphicsShaderProvider for RenderSlice<'_, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    for c in self.0 {
      c.build(builder);
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    for c in self.0.iter().rev() {
      c.post_build(builder);
    }
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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.as_slice().build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.as_slice().post_build(builder)
  }
}

#[derive(Default)]
pub struct RenderVec<'a>(Vec<Box<dyn RenderComponent + 'a>>);

impl<'a> RenderVec<'a> {
  pub fn with(mut self, c: impl RenderComponent + 'a) -> Self {
    self.0.push(Box::new(c));
    self
  }

  pub fn push(&mut self, c: impl RenderComponent + 'a) -> &mut Self {
    self.0.push(Box::new(c));
    self
  }

  pub fn as_slice(&self) -> impl RenderComponent + '_ {
    RenderSlice(self.0.as_slice())
  }
}

impl ShaderPassBuilder for RenderVec<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().setup_pass(ctx)
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.as_slice().post_setup_pass(ctx)
  }
}

impl ShaderHashProvider for RenderVec<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.as_slice().hash_pipeline(hasher)
  }
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.as_slice().hash_type_info(hasher)
  }
}

impl GraphicsShaderProvider for RenderVec<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.as_slice().build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.as_slice().post_build(builder)
  }
}

pub struct OptionRender<T>(pub Option<T>);

impl<T: ShaderHashProvider> ShaderHashProvider for OptionRender<T> {
  shader_hash_type_id! {OptionRender<()>}
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    if let Some(com) = &self.0 {
      com.hash_pipeline_with_type_info(hasher);
    }
  }
}

impl<T: ShaderPassBuilder> ShaderPassBuilder for OptionRender<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(com) = &self.0 {
      com.setup_pass(ctx);
    }
  }
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(com) = &self.0 {
      com.post_setup_pass(ctx);
    }
  }
}

impl<T: GraphicsShaderProvider> GraphicsShaderProvider for OptionRender<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if let Some(com) = &self.0 {
      com.build(builder);
    }
  }
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if let Some(com) = &self.0 {
      com.post_build(builder);
    }
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
    hasher.hash_type::<BindingController<()>>();
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
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let before = builder.set_binding_slot(self.target);
    self.inner.build(builder);
    builder.set_binding_slot(before);
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let before = builder.set_binding_slot(self.target);
    self.inner.post_build(builder);
    builder.set_binding_slot(before);
  }
}
