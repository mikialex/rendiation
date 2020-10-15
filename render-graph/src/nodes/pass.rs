use crate::{
  same_as_target, ContentSourceNodeBuilder, ContentUnit, NodeBuilder, RenderGraph,
  RenderGraphBackend, RenderGraphExecutor, RenderGraphGraphicsBackend, RenderGraphNode,
  RenderGraphNodeHandle, RenderTargetSize, TargetNodeBuilder, TargetNodeData,
};
use rendiation_ral::{ResourceManager, Viewport, RAL};
use std::{collections::HashSet, marker::PhantomData};

pub trait ImmediateRenderableContent<T: RAL> {
  fn render(&self, pass: &mut T::RenderPass, res: &ResourceManager<T>);
  fn prepare(&mut self, renderer: &mut T::Renderer, resource: &mut ResourceManager<T>);
}

pub struct PassNodeData<T: RenderGraphBackend> {
  pub name: String,
  pub(crate) viewport_modifier: Box<dyn Fn(RenderTargetSize) -> Viewport>,
  pub(crate) pass_op_modifier: Box<
    dyn Fn(
      <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
    ) -> <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
  >,
  pub(crate) input_targets_map: HashSet<RenderGraphNodeHandle<T>>,
  pub(crate) contents_to_render: Vec<RenderGraphNodeHandle<T>>,
  pub(crate) immediate_content_to_render: Vec<Box<dyn ImmediateRenderableContent<T::Graphics>>>,
  pub(crate) target_to: Option<RenderGraphNodeHandle<T>>,

  pub target_reuse_release_list: HashSet<RenderGraphNodeHandle<T>>,
  pub content_reuse_release_list: HashSet<RenderGraphNodeHandle<T>>,
}

impl<T: RenderGraphBackend> PassNodeData<T> {
  pub fn viewport(&self, target_size: RenderTargetSize) -> Viewport {
    (&self.viewport_modifier)(target_size)
  }

  pub fn execute(
    &self,
    self_handle: RenderGraphNodeHandle<T>,
    graph: &RenderGraph<T>,
    executor: &mut RenderGraphExecutor<T>,
    final_target: &<T::Graphics as RAL>::RenderTarget,
    renderer: &mut <T::Graphics as RAL>::Renderer,
    content_provider: &mut T::ContentProviderImpl,
  ) {
    let handle = self_handle;
    let mut graph = graph.graph.borrow_mut();

    let (real_size, pass_builder) = {
      let target_to = {
        let pass_data = graph.get_node_data_unwrap::<PassNodeData<_>>(handle);
        executor.working_content_unit.clear();

        let pool = &mut executor.content_pool;
        let extender = pass_data
          .contents_to_render
          .iter()
          .map(|&n| (n, pool.load(n)));
        executor.working_content_unit.extend(extender);
        pass_data.target_to.unwrap()
      };

      let target = graph
        .graph
        .get_node_mut(target_to)
        .data_mut()
        .downcast_mut::<TargetNodeData<_>>()
        .unwrap();

      let pass_builder = <T::Graphics as RenderGraphGraphicsBackend>::create_render_pass_builder(
        renderer,
        if target.is_final_target() {
          final_target
        } else {
          executor
            .target_pool
            .request_render_target(handle, target, renderer)
        },
      );

      (
        target.update_real_size(executor.current_final_size),
        pass_builder,
      )
    };

    let pass_data = graph.get_node_data_unwrap::<PassNodeData<_>>(handle);

    // todo
    // pass_data
    //   .immediate_content_to_render
    //   .iter()
    //   .for_each(|c| c.prepare(renderer, resource));

    let pass_builder = (pass_data.pass_op_modifier)(pass_builder);

    let mut render_pass =
      <T::Graphics as RenderGraphGraphicsBackend>::begin_render_pass(renderer, pass_builder);

    <T::Graphics as RenderGraphGraphicsBackend>::set_viewport(
      renderer,
      &mut render_pass,
      pass_data.viewport(real_size),
    );

    let pool = &mut executor.content_pool;
    executor.working_content_unit.drain().for_each(|(n, c)| {
      c.render_pass(&mut render_pass, content_provider);
      pool.store(n, c)
    });

    // pass_data
    //   .immediate_content_to_render
    //   .iter()
    //   .for_each(|c| c.render(&mut render_pass, resource));

    <T::Graphics as RenderGraphGraphicsBackend>::end_render_pass(renderer, render_pass);

    self.target_reuse_release_list.iter().for_each(|&n| {
      executor
        .target_pool
        .return_render_target(n, graph.get_node_data_unwrap::<TargetNodeData<_>>(n))
    });

    self.content_reuse_release_list.iter().for_each(|&n| {
      executor.content_pool.de_active(n);
    })
  }
}

pub struct PassNodeBuilder<'a, T: RenderGraphBackend> {
  pub(crate) builder: NodeBuilder<'a, T, PassNodeData<T>>,
}

impl<'a, T: RenderGraphBackend> PassNodeBuilder<'a, T> {
  pub fn define_pass_ops(
    self,
    modifier: impl Fn(
        <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder,
      ) -> <T::Graphics as RenderGraphGraphicsBackend>::RenderPassBuilder
      + 'static,
  ) -> Self {
    self
      .builder
      .mutate_data(|p| p.pass_op_modifier = Box::new(modifier));
    self
  }

  pub fn render_by(self, content: &ContentSourceNodeBuilder<'a, T>) -> Self {
    self.builder.connect_from(&self.builder);
    self.builder.mutate_data(|p| {
      p.contents_to_render.push(content.builder.handle);
    });
    self
  }

  pub fn render_immediate(
    self,
    content: impl ImmediateRenderableContent<T::Graphics> + 'static,
  ) -> Self {
    self.builder.mutate_data(|p| {
      p.immediate_content_to_render.push(Box::new(content));
    });
    self
  }

  pub fn viewport_modifier(
    self,
    modifier: impl Fn(RenderTargetSize) -> Viewport + 'static,
  ) -> Self {
    self
      .builder
      .mutate_data(|p| p.viewport_modifier = Box::new(modifier));
    self
  }

  pub fn depend(self, target: &TargetNodeBuilder<'a, T>) -> Self {
    self.builder.mutate_data(|p| {
      p.input_targets_map.insert(target.builder.handle);
    });
    self.builder.connect_from(&target.builder);
    self
  }
}

impl<T: RenderGraphBackend> RenderGraph<T> {
  pub fn pass(&self, name: &str) -> PassNodeBuilder<T> {
    let handle = self.create_node(RenderGraphNode::Pass(PassNodeData {
      name: name.to_owned(),
      viewport_modifier: Box::new(same_as_target),
      pass_op_modifier: Box::new(|b| b),
      input_targets_map: HashSet::new(),
      contents_to_render: Vec::new(),
      immediate_content_to_render: Vec::new(),
      target_to: None,
      target_reuse_release_list: HashSet::new(),
      content_reuse_release_list: HashSet::new(),
    }));
    PassNodeBuilder {
      builder: NodeBuilder {
        handle,
        graph: self,
        phantom: PhantomData,
      },
    }
  }
}
