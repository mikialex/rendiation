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

use rendiation_webgpu::*;

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait RenderComponent: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {
  fn render(&self, gpu: &GPU, ctx: &mut GPURenderPassCtx) {
    let mut hasher = PipelineHasher::default();
    self.hash_pipeline(&mut hasher);

    let pipeline = gpu
      .device
      .create_and_cache_render_pipeline(hasher, |device| {
        device
          .build_pipeline_by_shadergraph(self.build_self().unwrap())
          .unwrap()
      });

    ctx
      .binding
      .setup_pass(&mut ctx.pass, &gpu.device, &pipeline);
  }
}

impl<T> RenderComponent for T where T: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {}

pub trait RenderComponentAny: RenderComponent + ShaderHashProviderAny {}
impl<T> RenderComponentAny for T where T: RenderComponent + ShaderHashProviderAny {}

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
    self.contents.iter().for_each(|c| c.setup_pass(ctx))
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
    self.contents.iter().for_each(|c| c.build(builder).unwrap());
    Ok(())
  }
}

pub trait WebGPUMaterial: Clone + Any {
  type GPU: RenderComponentAny;
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
  ) -> &'a dyn RenderComponentAny;
  fn is_keep_mesh_shape(&self) -> bool;
}

impl<M: WebGPUMaterial> WebGPUSceneMaterial for Identity<M> {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    res.update_material(self, gpu, sub_res)
  }
  fn is_keep_mesh_shape(&self) -> bool {
    self.deref().is_keep_mesh_shape()
  }
}

type MaterialIdentityMapper<T> = IdentityMapper<<T as WebGPUMaterial>::GPU, T>;
impl GPUMaterialCache {
  pub fn update_material<M: WebGPUMaterial>(
    &mut self,
    m: &Identity<M>,
    gpu: &GPU,
    res: &mut GPUResourceSubCache,
  ) -> &M::GPU {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| Box::new(MaterialIdentityMapper::<M>::default()))
      .downcast_mut::<MaterialIdentityMapper<M>>()
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
