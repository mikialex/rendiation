use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
  rc::Rc,
};
pub mod bindable;
pub use bindable::*;
pub mod states;
use rendiation_texture::TextureSampler;
use rendiation_webgpu::GPU;
pub use states::*;

pub mod basic;
pub use basic::*;

pub mod env_background;
pub use env_background::*;

use rendiation_algebra::Mat4;

use crate::SceneTextureCube;

use super::{
  Camera, CameraBindgroup, MaterialHandle, Mesh, ReferenceFinalization, Scene, SceneTexture2D,
  TransformGPU, TypedMaterialHandle, ValueID, ViewerRenderPass, WatchedArena,
};

impl Scene {
  fn add_material_inner<M: Material + 'static, F: FnOnce(MaterialHandle) -> M>(
    &mut self,
    creator: F,
  ) -> MaterialHandle {
    self
      .materials
      .insert_with(|handle| Box::new(creator(handle)))
  }

  pub fn add_material<M>(&mut self, material: M) -> TypedMaterialHandle<M>
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    let handle = self.add_material_inner(|handle| MaterialCell::new(material, handle));
    TypedMaterialHandle {
      handle,
      ty: PhantomData,
    }
  }

  pub fn get_mut_material<M>(&mut self, handle: TypedMaterialHandle<M>) -> &mut M
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    &mut self
      .materials
      .get_mut(handle.handle)
      .unwrap()
      .as_any_mut()
      .downcast_mut::<MaterialCell<M>>()
      .unwrap()
      .material
  }

  pub fn get_material<M>(&self, handle: TypedMaterialHandle<M>) -> &M
  where
    M: MaterialCPUResource + 'static,
    M::GPU: MaterialGPUResource<Source = M>,
  {
    &self
      .materials
      .get(handle.handle)
      .unwrap()
      .as_any()
      .downcast_ref::<MaterialCell<M>>()
      .unwrap()
      .material
  }
}

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource: Clone {
  type GPU: MaterialGPUResource<Source = Self>;
  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU;
}

pub trait MaterialGPUResource: Sized {
  type Source: MaterialCPUResource<GPU = Self>;
  /// This Hook will be called before this material rendering(set_pass) happens if any change recorded.
  ///
  /// If return true, means the following procedure will use simple full refresh update logic:
  /// just rebuild the entire gpu resource. This is just for convenient case.
  /// You can also impl incremental update logic to improve performance in high dynamic scenario
  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    _bindgroup_changed: bool,
  ) -> bool {
    true
  }

  /// This Hook will be called before this material rendering(set_pass) happens.
  /// Users must request to prepare the real pipeline before render
  fn request_pipeline(
    &mut self,
    source: &Self::Source,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  );

  fn setup_pass<'a>(
    &'a self,
    _pass: &mut wgpu::RenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    // default do nothing
  }
}

pub struct MaterialCell<T>
where
  T: MaterialCPUResource,
{
  property_changed: bool,
  bindgroups_dirty: bool,
  material: T,
  last_material: Option<T>,
  gpu: Option<T::GPU>,
  handle: MaterialHandle,
}

impl<T: MaterialCPUResource> MaterialCell<T> {
  pub fn new(material: T, handle: MaterialHandle) -> Self {
    Self {
      property_changed: true,
      bindgroups_dirty: true,
      material,
      last_material: None,
      gpu: None,
      handle,
    }
  }

  fn refresh_cache(&mut self) {
    self.property_changed = false;
    self.bindgroups_dirty = false;
    self.last_material = self.material.clone().into();
  }
}

pub struct SceneMaterialRenderPrepareCtx<'a> {
  pub active_camera: &'a Camera,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_matrix: &'a Mat4<f32>,
  pub model_gpu: &'a TransformGPU,
  pub pipelines: &'a mut PipelineResourceManager,
  pub pass: &'a dyn ViewerRenderPass,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub textures: &'a mut WatchedArena<SceneTexture2D>,
  pub texture_cubes: &'a mut WatchedArena<SceneTextureCube>,
  pub samplers: &'a mut HashMap<TextureSampler, Rc<wgpu::Sampler>>,
  pub reference_finalization: &'a ReferenceFinalization,
}

impl<'a> SceneMaterialRenderPrepareCtx<'a> {
  pub fn pipeline_ctx(&mut self) -> (&mut PipelineResourceManager, PipelineCreateCtx<'a>) {
    (
      self.pipelines,
      PipelineCreateCtx {
        camera_gpu: self.camera_gpu,
        model_gpu: self.model_gpu,
        active_mesh: self.active_mesh,
        pass: self.pass,
      },
    )
  }
}

pub struct PipelineCreateCtx<'a> {
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub pass: &'a dyn ViewerRenderPass,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub pipelines: &'a PipelineResourceManager,
  pub camera_gpu: &'a CameraBindgroup,
  pub model_gpu: &'a TransformGPU,
  pub active_mesh: &'a Box<dyn Mesh>,
  pub pass: &'a dyn ViewerRenderPass,
}

pub trait Material {
  /// When material's referenced bindable resources(outer ubo, texture) reference has changed
  /// This will be called, and the implementation should dirty it's inner bindgroups
  fn on_ref_resource_changed(&mut self);
  fn update<'a>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a>);
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, ctx: &SceneMaterialPassSetupCtx<'a>);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> Material for MaterialCell<T>
where
  T: 'static,
  T: MaterialCPUResource,
  T::GPU: MaterialGPUResource<Source = T>,
{
  fn update<'a>(&mut self, gpu: &GPU, ctx: &mut SceneMaterialRenderPrepareCtx<'a>) {
    if let Some(self_gpu) = &mut self.gpu {
      if self.property_changed || self.bindgroups_dirty {
        if self_gpu.update(&self.material, gpu, ctx, self.bindgroups_dirty) {
          self.gpu = T::create(&mut self.material, self.handle, gpu, ctx).into();
        }
        self.refresh_cache();
      }
    } else {
      self.gpu = T::create(&mut self.material, self.handle, gpu, ctx).into();
      self.refresh_cache();
    }

    self
      .gpu
      .as_mut()
      .unwrap()
      .request_pipeline(&self.material, gpu, ctx);
  }
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    self.gpu.as_ref().unwrap().setup_pass(pass, ctx)
  }
  fn on_ref_resource_changed(&mut self) {
    self.bindgroups_dirty = true;
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn Any {
    self.property_changed = true;
    self
  }
}

pub type CommonPipelineCache = TopologyPipelineVariant<StatePipelineVariant<PipelineUnit>>;

pub struct CommonPipelineVariantKey(ValueID<MaterialStates>, wgpu::PrimitiveTopology);

impl AsRef<ValueID<MaterialStates>> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &ValueID<MaterialStates> {
    &self.0
  }
}

impl AsRef<wgpu::PrimitiveTopology> for CommonPipelineVariantKey {
  fn as_ref(&self) -> &wgpu::PrimitiveTopology {
    &self.1
  }
}

pub struct PipelineResourceManager {
  pub cache: HashMap<TypeId, Box<dyn Any>>,
}

impl PipelineResourceManager {
  pub fn new() -> Self {
    Self {
      cache: HashMap::new(),
    }
  }

  pub fn get_cache_mut<M: Any, C: Any + Default>(&mut self) -> &mut C {
    self
      .cache
      .entry(TypeId::of::<M>())
      .or_insert_with(|| Box::new(C::default()))
      .downcast_mut::<C>()
      .unwrap()
  }

  pub fn get_cache<M: Any, C: Any>(&self) -> &C {
    self
      .cache
      .get(&TypeId::of::<M>())
      .unwrap()
      .downcast_ref::<C>()
      .unwrap()
  }
}
