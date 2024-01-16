mod flat;
pub use flat::*;
mod physical_sg;
pub use physical_sg::*;
mod physical_mr;
pub use physical_mr::*;
mod utils;
pub use utils::*;

use crate::*;

pub type ReactiveMaterialGPUOf<T> = <T as WebGPUMaterial>::ReactiveGPU;

pub trait WebGPUMaterial: IncrementalBase {
  type ReactiveGPU: ReactiveRenderComponentSource;
  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU;

  fn is_transparent(&self) -> bool;
}

pub trait WebGPUSceneMaterial: Send + Sync {
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance>;
  fn is_transparent(&self) -> bool;
}

define_dyn_trait_downcaster_static!(WebGPUSceneMaterial);
pub fn register_webgpu_material_features<T>()
where
  T: AsRef<dyn WebGPUSceneMaterial> + AsMut<dyn WebGPUSceneMaterial> + 'static,
{
  get_dyn_trait_downcaster_static!(WebGPUSceneMaterial).register::<T>()
}

impl WebGPUSceneMaterial for MaterialEnum {
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance> {
    match self {
      Self::PhysicalSpecularGlossiness(m) => {
        let instance = PhysicalSpecularGlossinessMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::PhysicalSpecularGlossiness(instance)
      }
      Self::PhysicalMetallicRoughness(m) => {
        let instance = PhysicalMetallicRoughnessMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::PhysicalMetallicRoughness(instance)
      }
      Self::Flat(m) => {
        let instance = FlatMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::Flat(instance)
      }
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(WebGPUSceneMaterial)
        .downcast_ref(m.as_ref().as_any())?
        .create_scene_reactive_gpu(ctx)?,
    }
    .into()
  }

  fn is_transparent(&self) -> bool {
    match self {
      Self::PhysicalSpecularGlossiness(m) => m.read().deref().is_transparent(),
      Self::PhysicalMetallicRoughness(m) => m.read().deref().is_transparent(),
      Self::Flat(m) => m.read().deref().is_transparent(),
      Self::Foreign(m) => {
        if let Some(m) =
          get_dyn_trait_downcaster_static!(WebGPUSceneMaterial).downcast_ref(m.as_ref().as_any())
        {
          m.is_transparent()
        } else {
          false
        }
      }
    }
  }
}

impl<M> WebGPUSceneMaterial for IncrementalSignalPtr<M>
where
  M: WebGPUMaterial,
{
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance> {
    let instance = M::create_reactive_gpu(self, ctx);
    MaterialGPUInstance::Foreign(Box::new(instance) as Box<dyn ReactiveRenderComponentSource>)
      .into()
  }

  fn is_transparent(&self) -> bool {
    self.read().deref().is_transparent()
  }
}
impl<T: WebGPUMaterial> AsRef<dyn WebGPUSceneMaterial> for IncrementalSignalPtr<T> {
  fn as_ref(&self) -> &(dyn WebGPUSceneMaterial + 'static) {
    self
  }
}
impl<T: WebGPUMaterial> AsMut<dyn WebGPUSceneMaterial> for IncrementalSignalPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn WebGPUSceneMaterial + 'static) {
    self
  }
}

#[pin_project::pin_project(project = MaterialGPUInstanceProj)]
pub enum MaterialGPUInstance {
  PhysicalMetallicRoughness(ReactiveMaterialGPUOf<PhysicalMetallicRoughnessMaterial>),
  PhysicalSpecularGlossiness(ReactiveMaterialGPUOf<PhysicalSpecularGlossinessMaterial>),
  Flat(ReactiveMaterialGPUOf<FlatMaterial>),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
}

impl ReactiveRenderComponent for MaterialGPUInstance {
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    match self {
      Self::PhysicalMetallicRoughness(m) => {
        Box::pin(m.inner.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      Self::PhysicalSpecularGlossiness(m) => {
        Box::pin(m.inner.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      Self::Flat(m) => Box::pin(m.inner.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
      Self::Foreign(m) => m
        .as_reactive_component()
        .create_render_component_delta_stream(),
    }
  }
}

impl Stream for MaterialGPUInstance {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    match self.project() {
      MaterialGPUInstanceProj::PhysicalMetallicRoughness(m) => m.poll_next_unpin(cx),
      MaterialGPUInstanceProj::PhysicalSpecularGlossiness(m) => m.poll_next_unpin(cx),
      MaterialGPUInstanceProj::Flat(m) => m.poll_next_unpin(cx),
      MaterialGPUInstanceProj::Foreign(m) => m.poll_next_unpin(cx),
    }
  }
}

impl ShaderHashProvider for MaterialGPUInstance {
  #[rustfmt::skip]
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(self).hash(hasher);
    match self {
      Self::PhysicalMetallicRoughness(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::PhysicalSpecularGlossiness(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::Flat(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::Foreign(m) => m.as_reactive_component().hash_pipeline_with_type_info(hasher),
    }
  }
}

impl ShaderPassBuilder for MaterialGPUInstance {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::PhysicalMetallicRoughness(m) => m.as_reactive_component().setup_pass(ctx),
      Self::PhysicalSpecularGlossiness(m) => m.as_reactive_component().setup_pass(ctx),
      Self::Flat(m) => m.as_reactive_component().setup_pass(ctx),
      Self::Foreign(m) => m.as_reactive_component().setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::PhysicalMetallicRoughness(m) => m.as_reactive_component().post_setup_pass(ctx),
      Self::PhysicalSpecularGlossiness(m) => m.as_reactive_component().post_setup_pass(ctx),
      Self::Flat(m) => m.as_reactive_component().post_setup_pass(ctx),
      Self::Foreign(m) => m.as_reactive_component().post_setup_pass(ctx),
    }
  }
}

impl GraphicsShaderProvider for MaterialGPUInstance {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::PhysicalMetallicRoughness(m) => m.as_reactive_component().build(builder),
      Self::PhysicalSpecularGlossiness(m) => m.as_reactive_component().build(builder),
      Self::Flat(m) => m.as_reactive_component().build(builder),
      Self::Foreign(m) => m.as_reactive_component().build(builder),
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::PhysicalMetallicRoughness(m) => m.as_reactive_component().post_build(builder),
      Self::PhysicalSpecularGlossiness(m) => m.as_reactive_component().post_build(builder),
      Self::Flat(m) => m.as_reactive_component().post_build(builder),
      Self::Foreign(m) => m.as_reactive_component().post_build(builder),
    }
  }
}

pub type ReactiveMaterialRenderComponentDeltaSource = impl Stream<Item = RenderComponentDeltaFlag>;

impl GPUModelResourceCtx {
  pub fn get_or_create_reactive_material_render_component_delta_source(
    &self,
    material: &MaterialEnum,
  ) -> Option<ReactiveMaterialRenderComponentDeltaSource> {
    self
      .materials
      .write()
      .unwrap()
      .get_or_insert_with(material.guid()?, || {
        material.create_scene_reactive_gpu(&self.shared).unwrap()
      })
      .create_render_component_delta_stream()
      .into()
  }
}
