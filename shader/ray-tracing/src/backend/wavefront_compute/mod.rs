mod geometry;
pub use geometry::*;
mod sbt;
pub use sbt::*;
mod trace_task;
pub use trace_task::*;
mod pipeline;
pub use pipeline::*;

use crate::*;

pub struct GPUWaveFrontComputeRaytracingSystem;

impl GPURaytracingSystem for GPUWaveFrontComputeRaytracingSystem {
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider> {
    todo!()
  }

  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingPassEncoderProvider> {
    todo!()
  }

  fn create_acceleration_structure_system(
    &self,
  ) -> Box<dyn GPUAccelerationStructureSystemProvider> {
    // NaiveSahBVHSystem
    todo!()
  }
}

pub struct GPUWaveFrontComputeRaytracingDevice {
  gpu: GPU,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineDescriptor,
  ) -> Box<dyn GPURaytracingPipelineProvider> {
    Box::new(GPUWaveFrontComputeRaytracingBakedPipeline::compile(
      desc,
      &self.gpu.device,
      todo!(),
    ))
  }

  fn create_sbt(&self) -> Box<dyn ShaderBindingTableProvider> {
    Box::new(ShaderBindingTableInfo::new(todo!(), todo!()))
  }

  fn trace_op_base_builder(&self) -> RayCtxBaseBuilder {
    todo!()
  }
}

pub struct GPUWaveFrontComputeRaytracingEncoder {
  current_pipeline: Option<GPUWaveFrontComputeRaytracingBakedPipeline>,
}

impl RayTracingPassEncoderProvider for GPUWaveFrontComputeRaytracingEncoder {
  fn set_pipeline(&self, pipeline: &dyn GPURaytracingPipelineProvider) {
    todo!()
  }

  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup) {
    todo!()
  }

  fn trace_ray(&self, size: (u32, u32, u32), sbt: &dyn ShaderBindingTableProvider) {
    todo!()
  }
}

// pub struct TraceBase<T>(PhantomData<T>);

// impl<T> Default for TraceBase<T> {
//   fn default() -> Self {
//     Self(Default::default())
//   }
// }

// impl<T: Default + Copy + 'static> DeviceFutureProvider<T> for TraceBase<T> {
//   fn build_device_future(&self) -> DynDeviceFuture<T> {
//     BaseDeviceFuture::<T>::default().into_dyn()
//   }
// }
// impl<T, Cx> NativeRayTracingShaderBuilder<Cx, T> for TraceBase<T>
// where
//   T: Default,
//   Cx: NativeRayTracingShaderCtx,
// {
//   fn build(&self, _: &mut Cx) -> T {
//     T::default()
//   }
//   fn bind(&self, _: &mut BindingBuilder) {}
// }
