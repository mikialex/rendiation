use core::{
  any::Any,
  hash::Hash,
  pin::Pin,
  task::{Context, Poll},
};

use futures::{Stream, StreamExt};

use crate::*;

// todo, support single draw fallback, or if we miss inject the upper system, the shader dependency
// could not found.
#[pin_project::pin_project(project = MaybeBindlessMeshProj)]
pub enum MaybeBindlessMesh<T> {
  Traditional(T),
  Bindless(MeshSystemMeshInstance),
}

impl<T: ShaderHashProvider> ShaderHashProvider for MaybeBindlessMesh<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    match self {
      MaybeBindlessMesh::Traditional(t) => t.hash_pipeline(hasher),
      MaybeBindlessMesh::Bindless(t) => t.type_id().hash(hasher),
    }
  }
}
impl<T: GraphicsShaderProvider> GraphicsShaderProvider for MaybeBindlessMesh<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      MaybeBindlessMesh::Traditional(t) => t.build(builder),
      MaybeBindlessMesh::Bindless(_) => Ok(()),
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      MaybeBindlessMesh::Traditional(t) => t.post_build(builder),
      MaybeBindlessMesh::Bindless(_) => Ok(()),
    }
  }
}
impl<T: ShaderPassBuilder> ShaderPassBuilder for MaybeBindlessMesh<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      MaybeBindlessMesh::Traditional(t) => t.setup_pass(ctx),
      MaybeBindlessMesh::Bindless(_) => {}
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      MaybeBindlessMesh::Traditional(t) => t.post_setup_pass(ctx),
      MaybeBindlessMesh::Bindless(_) => {}
    }
  }
}

impl<T: Stream + Unpin> Stream for MaybeBindlessMesh<T> {
  type Item = T::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    match self.project() {
      MaybeBindlessMeshProj::Traditional(t) => t.poll_next_unpin(cx),
      MaybeBindlessMeshProj::Bindless(_) => Poll::Pending,
    }
  }
}

pub struct BindlessMeshProvider<'a, T> {
  pub base: &'a T,
  pub system: &'a BindlessMeshDispatcher,
}

impl<'a, T: ShaderPassBuilder> ShaderPassBuilder for BindlessMeshProvider<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.setup_pass(ctx);
    self.system.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.base.post_setup_pass(ctx)
  }
}

impl<'a, T: ShaderHashProviderAny> ShaderHashProviderAny for BindlessMeshProvider<'a, T> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher);
    self.base.hash_pipeline_with_type_info(hasher)
  }
}

impl<'a, T: ShaderHashProvider> ShaderHashProvider for BindlessMeshProvider<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.base.hash_pipeline(hasher);
    self.system.hash_pipeline(hasher)
  }
}
impl<'a, T: GraphicsShaderProvider> GraphicsShaderProvider for BindlessMeshProvider<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.base.build(builder)?;
    self.system.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.base.post_build(builder)
  }
}
