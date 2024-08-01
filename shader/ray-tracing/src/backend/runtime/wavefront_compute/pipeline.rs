use dyn_clone::DynClone;

use crate::*;

impl GPURaytracingPipelineBuilder {
  pub fn compile_task_executor(
    &self,
    device: &GPUDevice,
    init_size: usize,
  ) -> DeviceTaskGraphExecutor {
    let mut executor = DeviceTaskGraphExecutor::empty();

    executor.define_task(BaseDeviceFuture::default(), || (), device, init_size);

    for closet in &self.closest_hit_shaders {
      executor.define_task(BaseDeviceFuture::default(), || (), device, init_size);
    }

    for ray_gen in &self.ray_gen_shaders {
      executor.define_task(BaseDeviceFuture::default(), || (), device, init_size);
    }

    executor
  }
}

trait AnyPayload: DynClone + Any {}
dyn_clone::clone_trait_object!(AnyPayload);

trait TaskSpawnTarget {
  fn spawn(&self, payload: Box<dyn AnyPayload>) -> Node<u32>;
}

struct GPURayTraceTaskInvocationInstance {
  all_closest_hit_tasks: Vec<Box<dyn TaskSpawnTarget>>,
  all_missing_tasks: Vec<Box<dyn TaskSpawnTarget>>,
  acceleration_structure: NaiveSahBVHInvocationInstance,
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
