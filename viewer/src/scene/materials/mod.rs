use std::{
  any::Any,
  cell::{Cell, RefCell},
  hash::Hash,
  ops::{Deref, DerefMut},
  rc::Rc,
};
pub mod states;
pub use states::*;

pub mod wrapper;
pub use wrapper::*;

pub mod flat;
pub use flat::*;
pub mod line;
pub use line::*;
pub mod basic;
pub use basic::*;
pub mod fatline;
pub use fatline::*;
pub mod env_background;
pub use env_background::*;

use rendiation_webgpu::{
  BindGroupLayoutCache, GPURenderPass, PipelineBuilder, PipelineHasher, PipelineResourceCache,
  RenderPassInfo, GPU,
};

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource: Clone {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
}

pub trait MaterialGPUResource: Sized {
  type Source: MaterialCPUResource<GPU = Self>;

  /// This Hook will be called before this material rendering(set_pass)
  ///
  /// If return true, means the following procedure will use simple full refresh update logic:
  /// just rebuild the entire gpu resource. This is just for convenient case.
  /// You can also impl incremental update logic to improve performance in high dynamic scenario
  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    _bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> bool {
    true
  }

  fn hash_pipeline(&self, _source: &Self::Source, _hasher: &mut PipelineHasher) {}

  fn create_pipeline(
    &self,
    source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  );

  fn setup_pass_bindgroup<'a>(
    &self,
    _pass: &mut GPURenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx,
  ) {
    // default do nothing
  }
}

pub struct BindGroupDirtyWatcher {
  dirty: Cell<bool>,
}
impl Default for BindGroupDirtyWatcher {
  fn default() -> Self {
    Self {
      dirty: Cell::new(false),
    }
  }
}

impl BindGroupDirtyNotifier for BindGroupDirtyWatcher {
  fn notify_dirty(&self) {
    self.dirty.set(true);
  }
}

pub struct MaterialCell<T: MaterialCPUResource> {
  inner: Rc<RefCell<MaterialCellImpl<T>>>,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T) -> Self {
    let material = MaterialCellImpl::new(material);
    Self {
      inner: Rc::new(RefCell::new(material)),
    }
  }
}

impl<T: MaterialCPUResource> Clone for MaterialCell<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct MaterialCellImpl<T>
where
  T: MaterialCPUResource,
{
  property_changed: bool,
  material: T,
  current_pipeline: Option<Rc<wgpu::RenderPipeline>>,
  bindgroup_watcher: Rc<BindGroupDirtyWatcher>,
  last_material: Option<T>, // todo
  gpu: Option<T::GPU>,
}

impl<T: MaterialCPUResource> MaterialCellImpl<T> {
  pub fn new(material: T) -> Self {
    Self {
      property_changed: true,
      bindgroup_watcher: Default::default(),
      current_pipeline: None,
      material,
      last_material: None,
      gpu: None,
    }
  }

  fn refresh_cache(&mut self) {
    self.property_changed = false;
    self.bindgroup_watcher.dirty.set(false);
    self.last_material = self.material.clone().into();
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub model_info: Option<&'b TransformGPU>,
  pub active_mesh: Option<&'b dyn Mesh>,
  pub base: &'b mut SceneMaterialRenderPrepareCtxBase<'a>,
}

impl<'a, 'b> Deref for SceneMaterialRenderPrepareCtx<'a, 'b> {
  type Target = SceneMaterialRenderPrepareCtxBase<'a>;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}

impl<'a, 'b> DerefMut for SceneMaterialRenderPrepareCtx<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.base
  }
}

pub trait PassDispatcher: Any {
  fn build_pipeline(&self, builder: &mut PipelineBuilder);
}

pub struct DefaultPassDispatcher;
impl PassDispatcher for DefaultPassDispatcher {
  fn build_pipeline(&self, _builder: &mut PipelineBuilder) {}
}

pub struct SceneMaterialRenderPrepareCtxBase<'a> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub pass_info: &'a RenderPassInfo,
  pub pass: &'a dyn PassDispatcher,
  pub resources: &'a mut GPUResourceCache,
}

impl<'a, 'b> SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub fn pipeline_ctx(&mut self) -> (&mut PipelineResourceCache, PipelineCreateCtx) {
    (
      &mut self.base.resources.pipeline_resource,
      PipelineCreateCtx {
        layouts: &self.base.resources.layouts,
        active_mesh: self.active_mesh,
        pass_info: self.base.pass_info,
        pass: self.base.pass,
      },
    )
  }
}

pub struct PipelineCreateCtx<'a, 'b> {
  pub layouts: &'a BindGroupLayoutCache,
  pub active_mesh: Option<&'a dyn Mesh>,
  pub pass_info: &'b RenderPassInfo,
  pub pass: &'b dyn PassDispatcher,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub resources: &'a GPUResourceCache,
  pub model_gpu: Option<&'a TransformGPU>,
  pub active_mesh: Option<&'a dyn Mesh>,
  pub camera_gpu: &'a CameraBindgroup,
}

pub trait Material {
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>);
  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx);
  fn is_keep_mesh_shape(&self) -> bool;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Material for MaterialCellImpl<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>) {
    if let Some(self_gpu) = &mut self.gpu {
      if self.property_changed || self.bindgroup_watcher.dirty.get() {
        if self_gpu.update(&self.material, gpu, ctx, &self.bindgroup_watcher) {
          self.gpu = T::create(&mut self.material, gpu, ctx, &self.bindgroup_watcher).into();
        }
        self.refresh_cache();
      }
    } else {
      self.gpu = T::create(&mut self.material, gpu, ctx, &self.bindgroup_watcher).into();
      self.refresh_cache();
    }

    let topology = ctx.active_mesh.unwrap().topology();

    let mut hasher = Default::default();

    let m_gpu = self.gpu.as_mut().unwrap();

    self.material.type_id().hash(&mut hasher);
    ctx.pass_info.format_info.hash(&mut hasher);

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();

    pipeline_ctx.pass.type_id().hash(&mut hasher);
    m_gpu.hash_pipeline(&self.material, &mut hasher);

    self.current_pipeline = pipelines
      .get_or_insert_with(hasher, || {
        let mut builder = PipelineBuilder::default();
        builder.primitive_state.topology = topology;

        m_gpu.create_pipeline(&self.material, &mut builder, &gpu.device, &pipeline_ctx);
        pipeline_ctx.pass.build_pipeline(&mut builder);
        builder.build(&gpu.device)
      })
      .clone()
      .into();
  }

  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx) {
    let gpu = self.gpu.as_ref().unwrap();

    pass.set_pipeline_owned(self.current_pipeline.as_ref().unwrap());

    gpu.setup_pass_bindgroup(pass, ctx)
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.material.is_keep_mesh_shape()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self.property_changed = true;
    self
  }
}

impl<T> Material for MaterialCell<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a, 'b>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a, 'b>) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, ctx)
  }

  fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>, ctx: &SceneMaterialPassSetupCtx) {
    let inner = self.inner.borrow();
    inner.setup_pass(pass, ctx)
  }

  fn is_keep_mesh_shape(&self) -> bool {
    let inner = self.inner.borrow();
    inner.is_keep_mesh_shape()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    {
      let mut inner = self.inner.borrow_mut();
      inner.as_any_mut();
    }
    self
  }
}
