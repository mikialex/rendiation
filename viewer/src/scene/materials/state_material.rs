// use rendiation_webgpu::{PipelineRequester, PipelineResourceManager, GPU};

// use crate::*;

// #[derive(Clone)]
// pub struct FreeStateMaterial<T> {
//   pub material: T,
//   pub states: MaterialStates,
// }

// pub struct FreeStateMaterialGPU<T> {
//   state_id: ValueID<MaterialStates>,
//   gpu: T,
// }

// impl<T: MaterialGPUResource> PipelineRequester for FreeStateMaterialGPU<T> {
//   type Container = StatePipelineVariant<T::Container>;

//   type Key = ();
// }

// impl<T: MaterialGPUResource> MaterialGPUResource for FreeStateMaterialGPU<T> {
//   type Source = FreeStateMaterial<T::Source>;

//   fn pipeline_key(&self, source: &Self::Source, ctx: &PipelineCreateCtx) -> Self::Key {
//     todo!()
//   }

//   fn create_pipeline(
//     &self,
//     source: &Self::Source,
//     device: &wgpu::Device,
//     ctx: &PipelineCreateCtx,
//   ) -> wgpu::RenderPipeline {
//     todo!()
//   }
// }

// impl<T: MaterialCPUResource> MaterialCPUResource for FreeStateMaterial<T> {
//   type GPU = FreeStateMaterialGPU<T::GPU>;

//   fn create(
//     &mut self,
//     handle: MaterialHandle,
//     gpu: &GPU,
//     ctx: &mut SceneMaterialRenderPrepareCtx,
//   ) -> Self::GPU {
//     todo!()
//   }
// }
