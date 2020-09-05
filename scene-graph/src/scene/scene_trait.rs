use crate::RenderObject;
use rendiation_ral::*;

pub trait SceneBackend<T: RALBackend> {
  /// What data stored in tree node
  type NodeData: SceneNodeDataTrait<T>;
  /// Customized info stored directly on scene
  type SceneData: Default;
  fn render_object(
    object: RenderObject<T>,
    renderer: &mut T::Renderer,
    resources: &ResourceManager<T>,
  );
}

pub trait SceneNodeDataTrait<T: RALBackend>: Default {
  fn update_by_parent(&mut self, parent: Option<&Self>, resource: &mut ResourceManager<T>) -> bool;
  fn provide_render_object<U: Iterator<Item = RenderObject<T>>>(&self) -> U;
}
