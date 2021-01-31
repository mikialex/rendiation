use std::collections::HashMap;

use crate::{camera::VoxlandCamera, voxland::VoxlandState};
use rendiation_ral::{GeometryHandle, ResourceManager, ShadingHandle, ShadingProvider, RAL};
use rendiation_render_entity::Camera;
use rendiation_rendergraph::{
  ContentProvider, ImmediateRenderableContent, RenderGraph, RenderGraphBackend,
  RenderGraphExecutor, RenderTargetPool,
};
use rendiation_scenegraph::{
  default_impl::DefaultSceneBackend, Scene, SceneBackend, SceneDrawcallList, SceneRenderSource,
};
use rendiation_webgpu::{
  renderer::SwapChain, RenderTargetAble, ScreenRenderTarget, ScreenRenderTargetInstance,
  WGPURenderPassBuilder, WGPURenderer, WebGPU,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectConfig {
  enable_grain: bool,
}

pub struct VoxlandRenderer {
  cache: HashMap<EffectConfig, RenderGraph<DefaultRenderGraphBackend>>,
  executor: RenderGraphExecutor<DefaultRenderGraphBackend>,
  cached_drawcall_list: SceneDrawcallList<WebGPU>, // if use graph remove in future
}

struct DefaultContentProvider {
  scene: &'static mut Scene<WebGPU>,
  resource: &'static mut ResourceManager<WebGPU>,
}

impl SceneRenderSource<WebGPU, DefaultSceneBackend> for DefaultContentProvider {
  fn get_scene(&self) -> &Scene<WebGPU, DefaultSceneBackend> {
    &self.scene
  }
  fn get_resource(&self) -> &ResourceManager<WebGPU> {
    &self.resource
  }
}

impl ContentProvider<DefaultRenderGraphBackend> for DefaultContentProvider {
  fn get_source(
    &mut self,
    key: VoxlandSourceType,
    _: &RenderTargetPool<DefaultRenderGraphBackend>,
    _: &mut SceneDrawcallList<WebGPU>,
  ) {
    todo!()
  }
}

struct DefaultRenderGraphBackend;

impl RenderGraphBackend for DefaultRenderGraphBackend {
  type Graphics = WebGPU;
  type ContentProviderImpl = DefaultContentProvider;
  type ContentSourceKey = VoxlandSourceType;
  type ContentMiddleKey = ();
  type ContentUnitImpl = SceneDrawcallList<WebGPU>;
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum VoxlandSourceType {
  Main,
  Copier,
}

impl VoxlandRenderer {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
      executor: RenderGraphExecutor::new(),
      cached_drawcall_list: SceneDrawcallList::new(),
    }
  }

  fn render_experiment(
    &mut self,
    renderer: &mut WGPURenderer,
    target: &ScreenRenderTargetInstance,
    scene: &mut Scene<WebGPU>,
    resource: &mut ResourceManager<WebGPU>,
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

    let scene_main_content = graph.source(VoxlandSourceType::Main);

    let scene_pass = graph
      .pass("scene-pass")
      .define_pass_ops(|b: WGPURenderPassBuilder| {
        b.first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
          .depth(|d| d.load_with_clear(1.0).ok())
      })
      .render_by(&scene_main_content);

    let middle_target = graph.target("middle").draw_by_pass(&scene_pass);

    let copy_screen = graph.pass("copy_screen").depend(&middle_target);
    // .render_immediate(todo!());

    graph.finally().draw_by_pass(&copy_screen);
    graph
    // todo!()
  }

  pub fn render(
    &mut self,
    renderer: &mut WGPURenderer,
    scene: &mut Scene<WebGPU>,
    camera: &VoxlandCamera,
    resource: &mut ResourceManager<WebGPU>,
    output: &ScreenRenderTargetInstance,
  ) {
    let list = scene.update(resource, camera.camera(), &mut self.cached_drawcall_list);
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
