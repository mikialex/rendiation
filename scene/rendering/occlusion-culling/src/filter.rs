use crate::*;

pub fn filter_last_frame_visible_object(
  last_frame: &StorageBufferDataView<[Bool]>,
) -> Box<dyn AbstractCullerProvider> {
  Box::new(OnlyLastFrameVisible {
    last_frame: last_frame.clone(),
  })
}

#[derive(Clone)]
struct OnlyLastFrameVisible {
  last_frame: StorageBufferDataView<[Bool]>,
}

impl ShaderHashProvider for OnlyLastFrameVisible {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for OnlyLastFrameVisible {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(OnlyLastFrameVisibleInvocation {
      last_frame: cx.bind_by(&self.last_frame),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.last_frame);
  }
}

struct OnlyLastFrameVisibleInvocation {
  last_frame: ShaderPtrOf<[Bool]>,
}

impl AbstractCullerInvocation for OnlyLastFrameVisibleInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    id.less_than(self.last_frame.array_length())
      .select_branched(
        || self.last_frame.index(id).load().into_bool().not(),
        || val(true),
      )
  }
}
