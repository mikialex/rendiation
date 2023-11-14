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
