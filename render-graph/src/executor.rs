use rendiation_ral::RALBackend;

use crate::{
  build_pass_queue, RenderGraph, RenderGraphBackend, RenderGraphGraphicsBackend,
  RenderGraphNodeHandle, RenderTargetPool, RenderTargetSize,
};

pub(crate) struct PassExecuteInfo<T: RenderGraphBackend> {
  pub pass_node_handle: RenderGraphNodeHandle<T>,
  pub target_drop_list: Vec<RenderGraphNodeHandle<T>>,
}

pub struct RenderGraphExecutor<T: RenderGraphBackend> {
  target_pool: RenderTargetPool<T>,
  current_final_size: RenderTargetSize,
}

impl<'a, T: RenderGraphBackend> RenderGraphExecutor<T> {
  pub fn new() -> Self {
    Self {
      target_pool: RenderTargetPool::new(),
      current_final_size: RenderTargetSize::default(),
    }
  }

  pub fn force_clear_reuse_pool(&mut self, renderer: &<T::Graphics as RALBackend>::Renderer) {
    self.target_pool.clear_all(renderer)
  }

  pub fn render(
    &mut self,
    graph: &RenderGraph<T>,
    final_target: &<T::Graphics as RALBackend>::RenderTarget,
    renderer: &mut <T::Graphics as RALBackend>::Renderer,
    root_content_provider: &mut T::ContentProviderImpl,
  ) {
    let new_size = <T::Graphics as RenderGraphGraphicsBackend>::get_target_size(final_target);
    if self.current_final_size != new_size {
      self.force_clear_reuse_pool(renderer);
      self.current_final_size = new_size
    }

    let mut pass_queue = graph.pass_queue.borrow_mut();
    let queue = pass_queue.get_or_insert_with(|| build_pass_queue(graph));

    queue.iter().for_each(
      |PassExecuteInfo {
         pass_node_handle,
         target_drop_list,
       }| {
        let handle = *pass_node_handle;
        let mut graph = graph.graph.borrow_mut();

        let target_to_handle = *graph.get_node(handle).to().iter().next().unwrap();
        let target_to = graph.get_node_mut(target_to_handle).data_mut();

        let target_data = target_to
          .unwrap_target_data_mut()
          .update_real_size(self.current_final_size);
        let real_size = target_data.real_size();

        let pass_builder = <T::Graphics as RenderGraphGraphicsBackend>::create_render_pass_builder(
          renderer,
          if target_data.is_final_target() {
            final_target
          } else {
            self
              .target_pool
              .request_render_target(handle, target_data, renderer)
          },
        );

        let pass_data = graph.get_node_mut(handle).data_mut().unwrap_pass_data_mut();

        let pass_builder = (pass_data.pass_op_modifier)(pass_builder);

        let mut render_pass =
          <T::Graphics as RenderGraphGraphicsBackend>::begin_render_pass(renderer, pass_builder);

        <T::Graphics as RenderGraphGraphicsBackend>::set_viewport(
          renderer,
          &mut render_pass,
          pass_data.viewport(real_size),
        );

        // pass_data.render.as_mut().unwrap()(
        //   &self.target_pool,
        //   &mut content_provider,
        //   &mut render_pass,
        // ); // do render

        <T::Graphics as RenderGraphGraphicsBackend>::end_render_pass(renderer, render_pass);

        target_drop_list.iter().for_each(|&n| {
          self
            .target_pool
            .return_render_target(n, graph.get_node(n).data().unwrap_target_data())
        })
      },
    )
  }
}
