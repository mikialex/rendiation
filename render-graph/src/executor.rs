use std::collections::HashMap;

use rendiation_ral::RAL;

use crate::{
  build_pass_queue, ContentPool, RenderGraph, RenderGraphBackend, RenderGraphGraphicsBackend,
  RenderGraphNodeHandle, RenderTargetPool, RenderTargetSize,
};

pub struct RenderGraphExecutor<T: RenderGraphBackend> {
  pub(crate) target_pool: RenderTargetPool<T>,
  pub(crate) content_pool: ContentPool<T>,
  pub(crate) current_final_size: RenderTargetSize,
  pub(crate) working_content_unit: HashMap<RenderGraphNodeHandle<T>, T::ContentUnitImpl>,
}

impl<'a, T: RenderGraphBackend> RenderGraphExecutor<T> {
  pub fn new() -> Self {
    Self {
      target_pool: RenderTargetPool::new(),
      content_pool: ContentPool::new(),
      current_final_size: RenderTargetSize::default(),
      working_content_unit: HashMap::new(),
    }
  }

  pub fn force_clear_reuse_pool(&mut self, renderer: &<T::Graphics as RAL>::Renderer) {
    self.target_pool.clear_all(renderer)
  }

  pub fn render(
    &mut self,
    graph: &RenderGraph<T>,
    final_target: &<T::Graphics as RAL>::RenderTarget,
    renderer: &mut <T::Graphics as RAL>::Renderer,
    provider: &mut T::ContentProviderImpl,
  ) {
    let new_size = <T::Graphics as RenderGraphGraphicsBackend>::get_target_size(final_target);
    if self.current_final_size != new_size {
      self.force_clear_reuse_pool(renderer);
      self.current_final_size = new_size
    }

    let mut pass_queue = graph.pass_queue.borrow_mut();
    let queue = pass_queue.get_or_insert_with(|| build_pass_queue(graph));

    queue.iter().for_each(|&handle| {
      graph.graph.borrow().graph.get_node(handle).data().execute(
        handle,
        graph,
        self,
        provider,
        final_target,
        renderer,
      );
    })
  }
}
