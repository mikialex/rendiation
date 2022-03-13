use std::{
  any::{Any, TypeId},
  ops::Deref,
};
pub mod states;
pub use states::*;

pub mod wrapper;
pub use wrapper::*;

pub mod flat;
pub use flat::*;
// pub mod line;
// pub use line::*;
pub mod physical;
pub use physical::*;
pub mod fatline;
pub use fatline::*;
// pub mod env_background;
// pub use env_background::*;

use rendiation_webgpu::*;

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait ShaderPassBuilder {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx);
}

pub trait ShaderHashProvider {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
}

pub trait SourceOfRendering:
  ShaderHashProvider // able to get pipeline from cache at low cost
   + ShaderGraphProvider // able to provide shader logic and config pipeline
   + ShaderPassBuilder // able to bind resource to renderpass
{
}

impl<T> SourceOfRendering for T where T: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder
{}

pub trait WebGPUMaterial: Clone + Any {
  type GPU: SourceOfRendering;
  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait WebGPUSceneMaterial: 'static {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn SourceOfRendering;
  fn is_keep_mesh_shape(&self) -> bool;
}

impl<M: WebGPUMaterial> WebGPUSceneMaterial for ResourceWrapped<M> {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn SourceOfRendering {
    res.update_material(self, gpu, sub_res)
  }
  fn is_keep_mesh_shape(&self) -> bool {
    self.deref().is_keep_mesh_shape()
  }
}

type MaterialResourceMapper<T> = ResourceMapper<<T as WebGPUMaterial>::GPU, T>;
impl GPUMaterialCache {
  pub fn update_material<M: WebGPUMaterial>(
    &mut self,
    m: &ResourceWrapped<M>,
    gpu: &GPU,
    res: &mut GPUResourceSubCache,
  ) -> &M::GPU {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| Box::new(MaterialResourceMapper::<M>::default()))
      .downcast_mut::<MaterialResourceMapper<M>>()
      .unwrap();

    mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => ResourceLogicResult::Create(M::create_gpu(m, res, gpu)),
      ResourceLogic::Update(gpu_m, m) => {
        // todo check should really recreate?
        *gpu_m = M::create_gpu(m, res, gpu);
        ResourceLogicResult::Update(gpu_m)
      }
    })
  }
}

pub struct DefaultPassDispatcher;

impl ShaderHashProvider for DefaultPassDispatcher {}
impl ShaderPassBuilder for DefaultPassDispatcher {
  fn setup_pass(&self, _: &mut GPURenderPassCtx) {}
}

impl ShaderGraphProvider for DefaultPassDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder
      .bindgroups
      .uniform::<UniformBufferView<RenderPassGPUInfoData>>(SB::Pass);
    todo!()
  }
}
