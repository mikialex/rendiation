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
  type GPU: RenderComponentAny;
  fn create_gpu(&self, res: &mut ShareBindableResourceCtx, gpu: &GPU) -> Self::GPU;

  type ReactiveGPU;
  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU;
  fn as_material_gpu_instance(gpu: &Self::ReactiveGPU) -> &dyn MaterialGPUInstanceLike;

  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait WebGPUSceneMaterial: Send + Sync {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut ShareBindableResourceCtx,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

impl WebGPUSceneMaterial for SceneMaterialType {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut ShareBindableResourceCtx,
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
    sub_res: &mut ShareBindableResourceCtx,
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
    res: &mut ShareBindableResourceCtx,
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

pub trait MaterialGPUInstanceLike:
  Stream<Item = RenderComponentDeltaFlag> + RenderComponent + Unpin
{
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>;
}

#[pin_project::pin_project(project = MaterialGPUInstanceProj)]
pub enum MaterialGPUInstance {
  PhysicalMetallicRoughness(PhysicalMetallicRoughnessMaterialGPUReactive),
  PhysicalSpecularGlossiness(PhysicalSpecularGlossinessMaterialGPUReactive),
  Flat(FlatMaterialGPUReactive),
  Foreign(Box<dyn MaterialGPUInstanceLike>),
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
      MaterialGPUInstance::Foreign(m) => m.create_render_component_delta_stream(),
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
  ) -> MaterialReactive {
    self
      .materials
      .write()
      .unwrap()
      .get_or_insert_with(0, || match material {
        SceneMaterialType::PhysicalMetallicRoughness(m) => {
          let instance = PhysicalMetallicRoughnessMaterial::create_reactive_gpu(m, &self.shared);
          MaterialGPUInstance::PhysicalMetallicRoughness(instance)
        }
        SceneMaterialType::PhysicalSpecularGlossiness(m) => {
          let instance = PhysicalSpecularGlossinessMaterial::create_reactive_gpu(m, &self.shared);
          MaterialGPUInstance::PhysicalSpecularGlossiness(instance)
        }
        SceneMaterialType::Flat(m) => {
          let instance = FlatMaterial::create_reactive_gpu(m, &self.shared);
          MaterialGPUInstance::Flat(instance)
        }
        SceneMaterialType::Foreign(m) => {
          // if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMaterial>>() {
          //   m.check_update_gpu(res, sub_res, gpu)
          // } else {
          //   &()
          // }
          todo!()
        }
        _ => todo!(),
      })
      .create_render_component_delta_stream()
  }
}

// impl RenderComponent for MaterialGPUInstance {
// }
