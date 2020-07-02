use crate::{
  build_pass_queue, RenderGraph, RenderGraphBackend, RenderGraphNodeHandle, RenderTargetPool,
};

pub(crate) struct PassExecuteInfo<T: RenderGraphBackend> {
  pub pass_node_handle: RenderGraphNodeHandle<T>,
  pub target_drop_list: Vec<RenderGraphNodeHandle<T>>,
}

pub struct RenderGraphExecutor<'a, T: RenderGraphBackend> {
  renderer: &'a mut T::Renderer,
  target_pool: RenderTargetPool<T>,
  // yes i believe this should has same lifetime with renderer
  final_target: &'a T::RenderTarget,
}

impl<'a, T: RenderGraphBackend> RenderGraphExecutor<'a, T> {
  pub fn new(renderer: &'a mut T::Renderer, final_target: &'a T::RenderTarget) -> Self {
    Self {
      renderer,
      target_pool: RenderTargetPool::new(),
      final_target,
    }
  }

  pub fn render(&mut self, graph: &RenderGraph<T>) {
    let mut pass_queue = graph.pass_queue.borrow_mut();
    let queue = pass_queue.get_or_insert_with(|| build_pass_queue(graph));

    queue.iter().for_each(
      |PassExecuteInfo {
         pass_node_handle,
         target_drop_list,
       }| {
        let handle = *pass_node_handle;
        let mut graph = graph.graph.borrow_mut();

        // do target binding
        let target_to = graph.get_node_data(
          graph
            .get_node(*graph.get_node(handle).to().iter().next().unwrap())
            .data_handle(),
        );
        let target_data = target_to.unwrap_target_data();
        let mut render_pass = T::begin_render_pass(
          self.renderer,
          if target_data.is_final_target() {
            self.final_target
          } else {
            self
              .target_pool
              .request_render_target(handle, target_data, self.renderer)
          },
        );

        let pass_data = graph
          .get_node_data_mut_by_node(handle)
          .unwrap_pass_data_mut();

        pass_data.render.as_mut().unwrap()(&self.target_pool, &mut render_pass); // do render

        T::end_render_pass(self.renderer, render_pass);

        target_drop_list.iter().for_each(|n| {
          self
            .target_pool
            .return_render_target(*n, graph.get_node_data_by_node(*n).unwrap_target_data())
        })
      },
    )
  }
}
