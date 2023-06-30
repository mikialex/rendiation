use std::sync::Arc;

use incremental::EnumWrap;
use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;
use shadergraph::*;
use webgpu::*;

use crate::{FatLineMaterial, FatlineMesh};

mod axis;
mod camera;
mod grid;
mod ground;
pub use axis::*;
pub use camera::*;
pub use grid::*;
pub use ground::*;

pub type HelperLineMesh = FatlineMesh;
pub struct HelperLineModel {
  pub inner: SceneModelImpl,
}

impl HelperLineModel {
  pub fn new(material: FatLineMaterial, mesh: HelperLineMesh, node: &SceneNode) -> Self {
    let mat = material.into_ref();
    let mat = SceneMaterialType::Foreign(Arc::new(mat));

    let mesh = SceneMeshType::Foreign(Arc::new(mesh.into_ref()));

    let model = StandardModel::new(mat, mesh);
    let model = ModelType::Standard(model.into());
    let model = SceneModelImpl {
      model,
      node: node.clone(),
    };
    Self { inner: model }
  }

  pub fn update_mesh(&self, mesh: HelperLineMesh) {
    let mesh = SceneMeshType::Foreign(Arc::new(mesh.into_ref()));

    if let ModelType::Standard(model) = &self.inner.model {
      mesh.wrap(StandardModelDelta::mesh).apply_modify(model);
    }
  }
}

/// just add premultiplied alpha to shader
pub struct WidgetDispatcher {
  inner: DefaultPassDispatcher,
}

impl WidgetDispatcher {
  pub fn new(inner: DefaultPassDispatcher) -> Self {
    Self { inner }
  }
}

impl ShaderHashProvider for WidgetDispatcher {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.inner.hash_pipeline(hasher);
  }
}
impl ShaderPassBuilder for WidgetDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.setup_pass(ctx);
  }
}

impl ShaderGraphProvider for WidgetDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.build(builder)
  }
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.inner.post_build(builder)?;
    builder.fragment(|builder, _| {
      // todo improve, we should only override blend
      MaterialStates {
        blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        ..Default::default()
      }
      .apply_pipeline_builder(builder);

      let old = builder.get_fragment_out(0)?;
      let new = (old.xyz() * old.w(), old.w());
      builder.set_fragment_out(0, new)
    })
  }
}
