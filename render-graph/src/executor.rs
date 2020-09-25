use rendiation_ral::RALBackend;

use crate::{
  build_pass_queue, ContentPool, ContentProvider, ContentSourceNodeData, ContentUnit, PassNodeData,
  RenderGraph, RenderGraphBackend, RenderGraphGraphicsBackend, RenderGraphNodeHandle,
  RenderTargetPool, RenderTargetSize, TargetNodeData,
};

pub(crate) enum GraphExecutionInfo<T: RenderGraphBackend> {
  Pass(PassExecuteInfo<T>),
  SourceRetrieve(PassExecuteInfo<T>),
  ContentTransform(PassExecuteInfo<T>),
}

pub(crate) struct SourceRetrieveExecuteInfo<T: RenderGraphBackend> {
  pub node: RenderGraphNodeHandle<T>,
}

pub(crate) struct ContentTransformExecuteInfo<T: RenderGraphBackend> {
  pub node: RenderGraphNodeHandle<T>,
}

pub(crate) struct PassExecuteInfo<T: RenderGraphBackend> {
  pub node: RenderGraphNodeHandle<T>,
  pub target_reuse_release_list: Vec<RenderGraphNodeHandle<T>>,
  pub content_reuse_release_list: Vec<RenderGraphNodeHandle<T>>,
}

impl<T: RenderGraphBackend> PassExecuteInfo<T> {
  pub fn execute(
    &self,
    graph: &RenderGraph<T>,
    executor: &mut RenderGraphExecutor<T>,
    final_target: &<T::Graphics as RALBackend>::RenderTarget,
    renderer: &mut <T::Graphics as RALBackend>::Renderer,
    content_provider: &mut T::ContentProviderImpl,
  ) {
    let PassExecuteInfo {
      node,
      target_reuse_release_list,
      content_reuse_release_list,
    } = self;

    let handle = *node;
    let mut graph = graph.graph.borrow_mut();

    {
      let pass_data = graph
        .get_node(handle)
        .data()
        .downcast::<PassNodeData<_>>()
        .unwrap();
      executor.working_content_unit.clear();
      let pool = &executor.target_pool;
      let extender = pass_data
        .contents_to_render
        .iter()
        .map(|&n| {
          graph
            .get_node(n)
            .data()
            .downcast::<ContentSourceNodeData<_>>()
            .unwrap()
            .key
        })
        .map(|key| content_provider.get_source(key, pool));
      executor.working_content_unit.extend(extender);
    }

    let target_to_handle = *graph.get_node(handle).to().iter().next().unwrap();
    let target_to = graph.get_node_mut(target_to_handle).data_mut();

    let target_data = target_to
      .downcast_mut::<TargetNodeData<_>>()
      .unwrap()
      .update_real_size(executor.current_final_size);
    let real_size = target_data.real_size();

    let pass_builder = <T::Graphics as RenderGraphGraphicsBackend>::create_render_pass_builder(
      renderer,
      if target_data.is_final_target() {
        final_target
      } else {
        executor
          .target_pool
          .request_render_target(handle, target_data, renderer)
      },
    );

    let pass_data = graph
      .get_node(handle)
      .data()
      .downcast::<PassNodeData<_>>()
      .unwrap();

    let pass_builder = (pass_data.pass_op_modifier)(pass_builder);

    let mut render_pass =
      <T::Graphics as RenderGraphGraphicsBackend>::begin_render_pass(renderer, pass_builder);

    <T::Graphics as RenderGraphGraphicsBackend>::set_viewport(
      renderer,
      &mut render_pass,
      pass_data.viewport(real_size),
    );

    executor
      .working_content_unit
      .iter()
      .for_each(|i| i.render_pass(&mut render_pass, content_provider));

    <T::Graphics as RenderGraphGraphicsBackend>::end_render_pass(renderer, render_pass);

    target_reuse_release_list.iter().for_each(|&n| {
      executor.target_pool.return_render_target(
        n,
        graph
          .get_node(n)
          .data()
          .downcast::<TargetNodeData<_>>()
          .unwrap(),
      )
    })
  }
}

pub struct RenderGraphExecutor<T: RenderGraphBackend> {
  target_pool: RenderTargetPool<T>,
  content_pool: ContentPool<T>,
  current_final_size: RenderTargetSize,
  working_content_unit: Vec<T::ContentUnitImpl>,
}

impl<'a, T: RenderGraphBackend> RenderGraphExecutor<T> {
  pub fn new() -> Self {
    Self {
      target_pool: RenderTargetPool::new(),
      content_pool: ContentPool::new(),
      current_final_size: RenderTargetSize::default(),
      working_content_unit: Vec::new(),
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
    content_provider: &mut T::ContentProviderImpl,
  ) {
    let new_size = <T::Graphics as RenderGraphGraphicsBackend>::get_target_size(final_target);
    if self.current_final_size != new_size {
      self.force_clear_reuse_pool(renderer);
      self.current_final_size = new_size
    }

    let mut pass_queue = graph.pass_queue.borrow_mut();
    let queue = pass_queue.get_or_insert_with(|| build_pass_queue(graph));

    queue.iter().for_each(|info| {
      use GraphExecutionInfo::*;
      match info {
        Pass(info) => info.execute(graph, self, final_target, renderer, content_provider),
        SourceRetrieve(info) => todo!(),
        ContentTransform(info) => todo!(),
      }
    })
  }
}
