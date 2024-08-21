use dyn_clone::DynClone;

use crate::*;

pub struct TraceTaskImpl {
  payload_bumper: DeviceBumpAllocationInstance<u32>,
  tlas: Box<dyn GPUAccelerationStructureCompImplInstance>,
}

impl TracingTaskSpawner for TraceTaskImpl {
  fn spawn_new_tracing_task(
    &mut self,
    should_trace: Node<bool>,
    trace_call: ShaderRayTraceCall,
    payload: ShaderNodeRawHandle,
    payload_ty: ShaderSizedValueType,
  ) -> TaskFutureInvocationRightValue {
    todo!()
  }
}

trait AnyPayload: DynClone + Any {
  fn into_any(&self) -> Box<dyn Any>;
}
dyn_clone::clone_trait_object!(AnyPayload);

trait TaskSpawnTarget {
  fn spawn(&self, payload: Box<dyn AnyPayload>) -> Node<u32>;
}

// impl TaskSpawnTarget for TaskGroupDeviceInvocationInstance {
//   fn spawn(&self, payload: Box<dyn AnyPayload>) -> Node<u32> {
//     // self.spawn_new_task(payload.into_any().downcast().unwrap())
//     todo!()
//   }
// }

struct GPURayTraceTaskInvocationInstance {
  all_closest_hit_tasks: Vec<Box<dyn TaskSpawnTarget>>,
  all_missing_tasks: Vec<Box<dyn TaskSpawnTarget>>,
  acceleration_structure: Box<dyn GPUAccelerationStructureCompImplInvocationTraversable>,
}

fn spawn_dynamic(
  tasks: &[Box<dyn TaskSpawnTarget>],
  task_ty: Node<u32>,
  payload: Box<dyn AnyPayload>,
) -> Node<u32> {
  let mut switcher = switch_by(task_ty);
  let allocated_id = val(u32::MAX).make_local_var(); // todo error handling

  for (id, closet) in tasks.iter().enumerate() {
    switcher = switcher.case(id as u32, || {
      let allocated = closet.spawn(payload.clone());
      allocated_id.store(allocated);
    });
  }

  switcher.end_with_default(|| {});
  allocated_id.load()
}

impl GPURayTraceTaskInvocationInstance {
  pub fn spawn_closest(&self, task_ty: Node<u32>, payload: Box<dyn AnyPayload>) -> Node<u32> {
    spawn_dynamic(&self.all_closest_hit_tasks, task_ty, payload)
  }
  pub fn spawn_missing(&self, task_ty: Node<u32>, payload: Box<dyn AnyPayload>) -> Node<u32> {
    spawn_dynamic(&self.all_missing_tasks, task_ty, payload)
  }
}
