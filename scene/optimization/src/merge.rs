use crate::*;

pub struct SceneIncrementalMergeSystem {
  source_scene: Scene,
  optimized_scene: Scene,
  implementations: Box<dyn IncrementalMergeExt>,
}

pub trait IncrementalMergeExt {
  fn init(&mut self, source_scene: &Scene, target_scene: &Scene);
}

struct StandardAttributeModelIncrementalMergeSystem {
  mapping: usize,
  source_scene_table: Box<dyn DynamicReactiveCollection<NodeIdentity, Mat4<f32>>>,
}

// pub trait MaterialContentHash {
//   fn hash_material_content(&self, hasher: usize);
// }

// impl MaterialContentHash for MaterialEnum {
//   fn hash_material_content(&self, hasher: usize) {
//     match self {
//       MaterialEnum::PhysicalSpecularGlossiness(m) => todo!(),
//       MaterialEnum::PhysicalMetallicRoughness(m) => todo!(),
//       MaterialEnum::Flat(_) => todo!(),
//       MaterialEnum::Foreign(_) => todo!(),
//     }
//   }
// }

pub type MaterialGUID = u64;
pub type MaterialContentID = u64;

fn core_material_content_hash() -> impl ReactiveCollection<MaterialGUID, MaterialContentID> {}

pub type SceneModelGUID = u64;
pub type StandardModelGUID = u64;

fn sm_content_hash(
  foreign_materials_content_hash: impl FnOnce(
    Box<dyn DynamicReactiveCollection<StandardModelGUID, ()>>,
  ) -> Box<dyn DynamicReactiveCollection<MaterialGUID, ()>>,
  // relations
) -> impl ReactiveCollection<SceneModelGUID, MaterialContentID> {
  //
}
