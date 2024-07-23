use rendiation_webgpu::compute_shader_builder;

use crate::*;

pub struct RayTracingMegaKernelRuntime {}

impl GPURaytracingPipelineProvider for RayTracingMegaKernelRuntime {
  fn compile(&self, desc: GPURaytracingPipelineBuilder) -> u32 {
    compute_shader_builder()
      .config_work_group_size(64)
      .with_log_shader()
      .entry(|cx| {
        // let state = construct_state();
        // let ray = state. init_ray;
        loop_by(|_| {
          // if has ray
          let r = desc.geometry_provider.traverse(&|| {}, &|_| {});
          //

          if_by(r.is_some, || {
            // state.push_frame()
          })
          .else_by(|| {
            // state.pop_frame()
          });

          // barrier

          // state.poll()
          // update next ray
          //
          // barrier

          //  break loop if terminated
        });
        //
      });
    todo!()
  }
}

pub struct StackedStateMachine<T> {
  ctx_marker: PhantomData<T>,
  possible_states: Vec<Box<dyn Any>>,
}

pub struct StackedStateMachineInstance {
  trace_stack_state: WorkGroupSharedNode<[u8]>,
  trace_stack_info: WorkGroupSharedNode<[u32; 8]>,
}

impl<T> ShaderFuture for StackedStateMachine<T> {
  type State = StackedStateMachineInstance;
  type Output = ();
  type Ctx = T;
  fn reconstruct_state(&self, ctx: &mut Self::Ctx) -> Self::State {
    todo!()
  }

  fn poll(&self, state: &Self::State, ctx: &mut Self::Ctx) -> DeviceOption<Self::Output> {
    // ctx.get_payload_input::<T>();
    todo!()
  }
}
