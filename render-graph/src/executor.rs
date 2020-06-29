use crate::{
  build_pass_queue, RenderGraph, RenderGraphBackend, RenderGraphNodeHandle, RenderTargetPool,
};

pub(crate) struct PassExecuteInfo<T: RenderGraphBackend> {
  pub pass_node_handle: RenderGraphNodeHandle<T>,
  pub target_drop_list: Vec<RenderGraphNodeHandle<T>>,
}

pub struct RenderGraphExecutor<'a, T: RenderGraphBackend> {
  renderer: &'a T::Renderer,
  target_pool: &'a mut RenderTargetPool<T>,
}

impl<'a, T: RenderGraphBackend> RenderGraphExecutor<'a, T> {
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
        if target_data.is_screen() {
          T::set_render_target_screen(self.renderer);
        } else {
          T::set_render_target(
            self.renderer,
            self
              .target_pool
              .request_render_target(handle, target_data, self.renderer),
          )
        }

        let pass_data = graph
          .get_node_data_mut_by_node(handle)
          .unwrap_pass_data_mut();

        pass_data.render.as_mut().unwrap()(); // do render

        target_drop_list.iter().for_each(|n| {
          self
            .target_pool
            .return_render_target(*n, graph.get_node_data_by_node(*n).unwrap_target_data())
        })
      },
    )
  }
}
