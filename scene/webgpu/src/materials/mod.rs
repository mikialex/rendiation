pub mod states;
pub use states::*;
pub mod flat;
pub use flat::*;
pub mod physical_sg;
pub use physical_sg::*;
pub mod physical_mr;
pub use physical_mr::*;
pub mod fatline;
pub use fatline::*;
pub mod normal_mapping;
pub use normal_mapping::*;
pub mod utils;
pub use utils::*;

use crate::*;

pub trait WebGPUMaterial: Clone + Any + Incremental {
  type ReactiveGPU: AsMaterialGPUInstance;
  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU;

  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait AsMaterialGPUInstance: Stream<Item = RenderComponentDeltaFlag> + Unpin {
  fn as_material_gpu_instance(&self) -> &dyn MaterialGPUInstanceLike;
}

pub trait MaterialGPUInstanceLike: RenderComponent {
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>;
}

pub trait WebGPUSceneMaterial: Send + Sync {
  fn id(&self) -> Option<usize>;
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance>;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

impl WebGPUSceneMaterial for SceneMaterialType {
  fn id(&self) -> Option<usize> {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.id(),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.id(),
      SceneMaterialType::Flat(m) => m.id(),
      SceneMaterialType::Foreign(m) => {
        return if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          m.id()
        } else {
          None
        }
      }
      _ => return None,
    }
    .into()
  }
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance> {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => {
        let instance = PhysicalSpecularGlossinessMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::PhysicalSpecularGlossiness(instance)
      }
      SceneMaterialType::PhysicalMetallicRoughness(m) => {
        let instance = PhysicalMetallicRoughnessMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::PhysicalMetallicRoughness(instance)
      }
      SceneMaterialType::Flat(m) => {
        let instance = FlatMaterial::create_reactive_gpu(m, ctx);
        MaterialGPUInstance::Flat(instance)
      }
      SceneMaterialType::Foreign(m) => {
        return if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          m.create_scene_reactive_gpu(ctx)
        } else {
          None
        }
      }
      _ => return None,
    }
    .into()
  }

  fn is_keep_mesh_shape(&self) -> bool {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.read().deref().is_keep_mesh_shape(),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.read().deref().is_keep_mesh_shape(),
      SceneMaterialType::Flat(m) => m.read().deref().is_keep_mesh_shape(),
      SceneMaterialType::Foreign(m) => {
        if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          m.is_keep_mesh_shape()
        } else {
          true
        }
      }
      _ => true,
    }
  }
  fn is_transparent(&self) -> bool {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.read().deref().is_transparent(),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.read().deref().is_transparent(),
      SceneMaterialType::Flat(m) => m.read().deref().is_transparent(),
      SceneMaterialType::Foreign(m) => {
        if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          m.is_transparent()
        } else {
          false
        }
      }
      _ => false,
    }
  }
}

impl<M: WebGPUMaterial + Send + Sync> WebGPUSceneMaterial for SceneItemRef<M> {
  fn id(&self) -> Option<usize> {
    self.id().into()
  }
  fn create_scene_reactive_gpu(
    &self,
    ctx: &ShareBindableResourceCtx,
  ) -> Option<MaterialGPUInstance> {
    let instance = M::create_reactive_gpu(self, ctx);
    MaterialGPUInstance::Foreign(Box::new(instance) as Box<dyn AsMaterialGPUInstance>).into()
  }

  fn is_keep_mesh_shape(&self) -> bool {
    self.read().deref().is_keep_mesh_shape()
  }

  fn is_transparent(&self) -> bool {
    self.read().deref().is_transparent()
  }
}
#[pin_project::pin_project(project = MaterialGPUInstanceProj)]
pub enum MaterialGPUInstance {
  PhysicalMetallicRoughness(PhysicalMetallicRoughnessMaterialGPUReactive),
  PhysicalSpecularGlossiness(PhysicalSpecularGlossinessMaterialGPUReactive),
  Flat(FlatMaterialGPUReactive),
  Foreign(Box<dyn AsMaterialGPUInstance>),
}

impl MaterialGPUInstance {
  pub fn create_render_component_delta_stream(
    &self,
  ) -> impl Stream<Item = RenderComponentDeltaFlag> + Unpin {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => {
        Box::pin(m.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => {
        Box::pin(m.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      MaterialGPUInstance::Flat(m) => Box::pin(m.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
      MaterialGPUInstance::Foreign(m) => m
        .as_material_gpu_instance()
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

pub type MaterialReactive = impl Stream<Item = RenderComponentDeltaFlag>;

impl GPUModelResourceCtx {
  pub fn get_or_create_reactive_material_gpu(
    &self,
    material: &SceneMaterialType,
  ) -> Option<MaterialReactive> {
    self
      .materials
      .write()
      .unwrap()
      .get_or_insert_with(material.id()?, || {
        material.create_scene_reactive_gpu(&self.shared).unwrap()
      })
      .create_render_component_delta_stream()
      .into()
  }
}

impl ShaderHashProvider for MaterialGPUInstance {
  #[rustfmt::skip]
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
      MaterialGPUInstance::Flat(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
      MaterialGPUInstance::Foreign(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
    }
  }
}

impl ShaderPassBuilder for MaterialGPUInstance {
  #[rustfmt::skip]
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => m.as_material_gpu_instance().setup_pass(ctx),
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => m.as_material_gpu_instance().setup_pass(ctx),
      MaterialGPUInstance::Flat(m) => m.as_material_gpu_instance().setup_pass(ctx),
      MaterialGPUInstance::Foreign(m) => m.as_material_gpu_instance().setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    #[rustfmt::skip]
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
      MaterialGPUInstance::Flat(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
      MaterialGPUInstance::Foreign(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
    }
  }
}

impl ShaderGraphProvider for MaterialGPUInstance {
  #[rustfmt::skip]
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => m.as_material_gpu_instance().build(builder),
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => m.as_material_gpu_instance().build(builder),
      MaterialGPUInstance::Flat(m) => m.as_material_gpu_instance().build(builder),
      MaterialGPUInstance::Foreign(m) => m.as_material_gpu_instance().build(builder),
    }
  }

  #[rustfmt::skip]
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => m.as_material_gpu_instance().post_build(builder),
      MaterialGPUInstance::PhysicalSpecularGlossiness(m) => m.as_material_gpu_instance().post_build(builder),
      MaterialGPUInstance::Flat(m) => m.as_material_gpu_instance().post_build(builder),
      MaterialGPUInstance::Foreign(m) => m.as_material_gpu_instance().post_build(builder),
    }
  }
}
