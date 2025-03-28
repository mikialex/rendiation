use rendiation_shader_api::*;

pub trait ShaderProducerInvocation {
  /// this must be static to shader runtime, in u32 count
  fn work_memory_size(&self) -> u32;
  fn has_work(&self) -> Node<bool>;
  fn produce(&self, parent_id: Node<u32>, address: Node<u32>, memory: &ShaderPtrOf<[u32]>);
}

pub trait ShaderConsumerInvocation {
  fn consume(&self, address: Node<u32>, memory: &ShaderPtrOf<[u32]>);
}

struct ShaderMPMCInvocation {
  workgroup_width: u32,
  // this is in shared memory
  scratch_space: ShaderPtrOf<[u32]>,
  // this is in shared memory
  finished_count: ShaderPtrOf<DeviceAtomic<u32>>,
  // this is in shared memory
  new_spawn_count: ShaderPtrOf<DeviceAtomic<u32>>,
  producer: Box<dyn ShaderProducerInvocation>,
  consumer: Box<dyn ShaderConsumerInvocation>,
}

impl ShaderMPMCInvocation {
  fn try_bump(&self) -> (Node<u32>, Node<bool>) {
    let u32_count = val(self.producer.work_memory_size());
    todo!()
  }

  fn reset_bumper(&self) {
    //
  }

  fn compute(&self, cx: &mut ShaderComputePipelineBuilder) {
    let producer_terminated = val(false).make_local_var();
    let self_id = cx.local_invocation_id().x();
    loop_by(|lp| {
      if_by(producer_terminated.load().not(), || {
        loop_by(|spawn_lp| {
          if_by(self.producer.has_work().not(), || {
            producer_terminated.store(true);
            self.finished_count.atomic_add(val(1));
            spawn_lp.do_break();
          });
          let (address, success) = self.try_bump();
          if_by(success, || {
            self.producer.produce(self_id, address, &self.scratch_space);
          })
          .else_by(|| {
            spawn_lp.do_break();
          });
        });
      });

      workgroup_barrier(); // make sure the task is visible to consume

      let new_work_todo = self.new_spawn_count.atomic_load();
      let iter_count = new_work_todo / val(self.workgroup_width);

      iter_count.into_shader_iter().for_each(|loop_index, _| {
        let work_id = loop_index * val(self.workgroup_width);
        if_by(work_id.less_than(new_work_todo), || {
          let address = val(self.producer.work_memory_size()) * work_id;
          self.consumer.consume(address, &self.scratch_space);
        });
      });

      workgroup_barrier(); // make sure the task has consumed
      self.new_spawn_count.atomic_store(val(0));
      self.reset_bumper();

      // todo, use workgroupUniformLoad(), or uniformity is invalid
      let finished_count = self.finished_count.atomic_load();
      if_by(finished_count.equals(self.workgroup_width), || {
        lp.do_break();
      });
    });
  }
}
