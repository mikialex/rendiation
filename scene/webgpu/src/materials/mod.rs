pub mod states;
use std::sync::Arc;

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

use crate::*;

pub trait WebGPUMaterial: Clone + Any + Incremental {
  type GPU: RenderComponentAny;
  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait WebGPUSceneMaterial: Send + Sync {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

impl WebGPUSceneMaterial for SceneMaterialType {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.check_update_gpu(res, sub_res, gpu),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.check_update_gpu(res, sub_res, gpu),
      SceneMaterialType::Flat(m) => m.check_update_gpu(res, sub_res, gpu),
      SceneMaterialType::Foreign(m) => {
        if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          m.check_update_gpu(res, sub_res, gpu)
        } else {
          &()
        }
      }
      _ => &(),
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    match self {
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.is_keep_mesh_shape(),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.is_keep_mesh_shape(),
      SceneMaterialType::Flat(m) => m.is_keep_mesh_shape(),
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
      SceneMaterialType::PhysicalSpecularGlossiness(m) => m.is_transparent(),
      SceneMaterialType::PhysicalMetallicRoughness(m) => m.is_transparent(),
      SceneMaterialType::Flat(m) => m.is_transparent(),
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
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    res.update_material(&self.read(), gpu, sub_res)
  }
  fn is_keep_mesh_shape(&self) -> bool {
    self.read().deref().is_keep_mesh_shape()
  }

  fn is_transparent(&self) -> bool {
    self.read().deref().is_transparent()
  }
}

type MaterialIdentityMapper<T> = IdentityMapper<<T as WebGPUMaterial>::GPU, T>;
impl GPUMaterialCache {
  pub fn update_material<M: WebGPUMaterial>(
    &mut self,
    m: &Identity<M>,
    gpu: &GPU,
    res: &mut GPUResourceSubCache,
  ) -> &M::GPU {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| Box::<MaterialIdentityMapper<M>>::default())
      .downcast_mut::<MaterialIdentityMapper<M>>()
      .unwrap();

    mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => ResourceLogicResult::Create(M::create_gpu(m, res, gpu)),
      ResourceLogic::Update(gpu_m, m) => {
        // todo check should really recreate?
        *gpu_m = M::create_gpu(m, res, gpu);
        ResourceLogicResult::Update(gpu_m)
      }
    })
  }
}

pub enum MaterialGPUInstance {
  PhysicalMetallicRoughness(PhysicalMetallicRoughnessMaterialGPUReactive),
  // PhysicalMetallicRoughness(SceneItemRef<PhysicalMetallicRoughnessMaterial>),
  // Flat(SceneItemRef<FlatMaterial>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

impl MaterialGPUInstance {
  pub fn create_render_component_delta_stream(&self) -> impl Stream<Item = RenderComponentDelta> {
    match self {
      MaterialGPUInstance::PhysicalMetallicRoughness(m) => {
        m.as_ref().create_render_component_delta_stream()
      }
      MaterialGPUInstance::Foreign(_) => todo!(),
    }
  }
}

impl Stream for MaterialGPUInstance {
  type Item = RenderComponentDelta;

  fn poll_next(
    self: __core::pin::Pin<&mut Self>,
    cx: &mut task::Context<'_>,
  ) -> task::Poll<Option<Self::Item>> {
    todo!()
  }
}

impl GlobalGPUSystemModelContentView {
  pub fn get_or_create_reactive_material_gpu(
    &self,
    material: &SceneMaterialType,
  ) -> impl Stream<Item = RenderComponentDelta> {
    self
      .materials
      .write()
      .unwrap()
      .get_or_insert_with(0, || match material {
        SceneMaterialType::PhysicalMetallicRoughness(material) => {
          let instance = physical_metallic_roughness_material_build_gpu(material, &self.shared);
          MaterialGPUInstance::PhysicalMetallicRoughness(instance)
        }
        SceneMaterialType::PhysicalSpecularGlossiness(_) => todo!(),
        SceneMaterialType::Flat(_) => todo!(),
        SceneMaterialType::Foreign(_) => todo!(),
        _ => todo!(),
      })
      .create_render_component_delta_stream()
  }
}

// impl RenderComponent for MaterialGPU {
// }
