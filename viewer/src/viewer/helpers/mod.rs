use std::sync::Arc;

use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;
use shadergraph::*;
use webgpu::*;

pub mod axis;
pub mod camera;
pub mod grid;
pub mod ground;

pub type HelperLineMesh = FatlineMesh;
pub struct HelperLineModel {
  pub inner: SceneModelImpl,
}

impl HelperLineModel {
  pub fn new(material: FatLineMaterial, mesh: HelperLineMesh, node: &SceneNode) -> Self {
    let mat = SceneItemRef::new(material.use_state_helper_like());
    let mat: Box<dyn WebGPUSceneMaterial> = Box::new(mat);
    let mat = SceneMaterialType::Foreign(Arc::new(mat));

    let mesh: Box<dyn WebGPUSceneMesh> = Box::new(SceneItemRef::new(mesh));
    let mesh = SceneMeshType::Foreign(Arc::new(mesh));

    let model = StandardModel {
      material: mat.into(),
      mesh: mesh.into(),
      group: Default::default(),
    };
    let model = SceneModelType::Standard(model.into());
    let model = SceneModelImpl {
      model,
      node: node.clone(),
    };
    Self { inner: model }
  }

  pub fn update_mesh(&self, mesh: HelperLineMesh) {
    let mesh: Box<dyn WebGPUSceneMesh> = Box::new(SceneItemRef::new(mesh));
    let mesh = SceneMeshType::Foreign(Arc::new(mesh));

    if let SceneModelType::Standard(model) = &self.inner.model {
      model.write().mesh = mesh.into();
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

struct HelperMesh {
  // inner:
}
