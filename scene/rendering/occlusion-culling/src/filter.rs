use crate::*;

pub fn filter_last_frame_visible_object(
  last_frame_invisible: &StorageBufferDataView<[Bool]>,
) -> Box<dyn AbstractCullerProvider> {
  Box::new(OnlyLastFrameVisible {
    last_frame_invisible: last_frame_invisible.clone(),
  })
}

#[derive(Clone)]
struct OnlyLastFrameVisible {
  last_frame_invisible: StorageBufferDataView<[Bool]>,
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
      last_frame_invisible: cx.bind_by(&self.last_frame_invisible),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.last_frame_invisible);
  }
}

struct OnlyLastFrameVisibleInvocation {
  last_frame_invisible: ShaderPtrOf<[Bool]>,
}

impl AbstractCullerInvocation for OnlyLastFrameVisibleInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    id.less_than(self.last_frame_invisible.array_length())
      .select_branched(
        || self.last_frame_invisible.index(id).load().into_bool(),
        || val(true),
      )
  }
}
