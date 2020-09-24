use std::collections::HashMap;

use crate::rinecraft::RinecraftState;
use rendiation_ral::ResourceManager;
use rendiation_rendergraph::{
  ContentProvider, RenderGraph, RenderGraphBackend, RenderGraphExecutor, RenderTargetPool,
};
use rendiation_scenegraph::{default_impl::DefaultSceneBackend, DrawcallList, Scene};
use rendiation_webgpu::{
  renderer::SwapChain, RenderTargetAble, ScreenRenderTarget, ScreenRenderTargetInstance,
  WGPURenderPassBuilder, WGPURenderer,
};
use rendium::EventCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectConfig {
  enable_grain: bool,
}

pub struct RinecraftRenderer {
  cache: HashMap<EffectConfig, RenderGraph<DefaultRenderGraphBackend>>,
  executor: RenderGraphExecutor<DefaultRenderGraphBackend>,
}

struct DefaultRenderGraphBackend;

impl RenderGraphBackend for DefaultRenderGraphBackend {
  type Graphics = WGPURenderer;
  type ContentProviderImpl = DefaultContentProvider;
  type ContentKey = RinecraftSourceType;
  type ContentUnitImpl = DrawcallList<WGPURenderer>;
}

struct DefaultContentProvider {
  scene: &'static mut Scene<WGPURenderer>,
  resource: &'static mut ResourceManager<WGPURenderer>,
}

#[derive(Copy, Clone, Debug)]
pub enum RinecraftSourceType {}

impl ContentProvider<DefaultRenderGraphBackend> for DefaultContentProvider {
  fn get_source(
    &mut self,
    key: RinecraftSourceType,
    _: &RenderTargetPool<DefaultRenderGraphBackend>,
  ) -> DrawcallList<WGPURenderer> {
    todo!()
  }
}

impl RinecraftRenderer {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
      executor: RenderGraphExecutor::new(),
    }
  }

  fn render_experiment(
    &mut self,
    renderer: &mut WGPURenderer,
    target: &ScreenRenderTargetInstance,
    scene: &mut Scene<WGPURenderer>,
    resource: &mut ResourceManager<WGPURenderer>,
    config: &EffectConfig,
  ) {
    let graph = self
      .cache
      .entry(*config)
      .or_insert_with(|| Self::build(config));

    let scene = unsafe { std::mem::transmute(scene) };
    let resource = unsafe { std::mem::transmute(resource) };
    let target = unsafe { std::mem::transmute(&target) };
    let mut content = DefaultContentProvider { scene, resource };

    self.executor.render(graph, target, renderer, &mut content);
  }

  fn build(config: &EffectConfig) -> RenderGraph<DefaultRenderGraphBackend> {
    let graph = RenderGraph::new();

    // let normal_pass = graph.pass("normal");
    // let normal_target = graph.target("normal").from_pass(&normal_pass);
    // let copy_screen = graph
    //   .pass("copy_screen")
    //   .depend(&normal_target)
    //   .render_by(|_, _| {
    //     let _a = 1;
    //   });

    let scene_pass = graph
      .pass("scene-pass")
      .define_pass_ops(|b: WGPURenderPassBuilder| {
        b.first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
          .depth(|d| d.load_with_clear(1.0).ok())
      });
    // .render_by(|_, _, pass| {
    //   todo!();
    //   todo!()
    // });

    graph.finally().from_pass(&scene_pass);
    graph
  }

  pub fn render(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WGPURenderer>,
    resource: &mut ResourceManager<WGPURenderer>,
    output: &ScreenRenderTargetInstance,
  ) {
    let list = scene.update(resource);
    resource.maintain_gpu(renderer);

    {
      let mut pass = output
        .create_render_pass_builder()
        .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
        .depth(|d| d.load_with_clear(1.0).ok())
        .create(renderer);

      list.render(unsafe { std::mem::transmute(&mut pass) }, scene, resource);
    }

    renderer.submit();
  }
}
