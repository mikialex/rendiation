use arena::Handle;

pub mod background;
// pub mod culling;
pub mod default_impl;
pub mod node;
pub mod render_unit;

pub use background::*;
// pub use culling::*;
pub use node::*;
pub use render_unit::*;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

use super::node::SceneNode;
use crate::{default_impl::DefaultSceneBackend, RALBackend, RenderObject};
use arena::*;
use arena_tree::*;
use rendiation_mesh_buffer::geometry::IntoExactSizeIterator;
use rendiation_ral::ResourceManager;

pub trait SceneBackend<T: RALBackend>: Sized {
  /// What data stored in tree node
  type NodeData: SceneNodeDataTrait<T>;
  /// Customized info stored directly on scene.
  /// Implementor could put extra effect struct, like background on it
  /// and take care of the rendering and updating.
  type SceneData: Default;
}

pub fn render_list<T: RALBackend, S: SceneBackend<T>>(
  raw_list: &Vec<Drawcall<T, S>>,
  pass: &mut T::RenderPass,
  scene: &Scene<T, S>,
  resources: &ResourceManager<T>,
) {
  raw_list.iter().for_each(|d| {
    let render_object = scene.render_objects.get(d.render_object).unwrap();
    T::render_object(&render_object, pass, resources);
  })
}

pub trait SceneNodeDataTrait<T: RALBackend>: Default {
  type RenderObjectIntoIterType;
  fn update_by_parent(&mut self, parent: Option<&Self>, resource: &mut ResourceManager<T>) -> bool;
  fn provide_render_object<'a>(&self) -> &Self::RenderObjectIntoIterType;
}

pub struct SceneNodeDataRenderObjectsProvider<'a, P>(pub &'a P);

pub struct Scene<T: RALBackend, S: SceneBackend<T> = DefaultSceneBackend> {
  pub render_objects: Arena<RenderObject<T>>,
  pub(crate) nodes: ArenaTree<S::NodeData>,
  pub scene_data: S::SceneData,
  cached_raw_drawcall_list: DrawcallList<T, S>,
  reused_traverse_stack: Vec<SceneNodeHandle<T, S>>,
}

impl<T: RALBackend, S: SceneBackend<T>> Scene<T, S> {
  pub fn new() -> Self {
    Self {
      render_objects: Arena::new(),
      nodes: ArenaTree::new(S::NodeData::default()),
      scene_data: S::SceneData::default(),
      cached_raw_drawcall_list: DrawcallList::new(),
      reused_traverse_stack: Vec::new(),
    }
  }

  pub fn update(&mut self, resources: &mut ResourceManager<T>) -> &DrawcallList<T, S>
  where
    for<'a> &'a <S::NodeData as SceneNodeDataTrait<T>>::RenderObjectIntoIterType:
      IntoExactSizeIterator<Item = &'a RenderObjectHandle<T>>,
    // maybe we could let SceneNodeDataTrait impl IntoExactSizeIterator for simplicity
  {
    // todo change detection and skip
    let root = self.get_root().handle();
    let list = &mut self.cached_raw_drawcall_list;
    list.inner.clear();
    self.nodes.traverse(
      root,
      &mut self.reused_traverse_stack,
      |this: &mut SceneNode<T, S>, parent: Option<&mut SceneNode<T, S>>| {
        let this_handle = this.handle();
        let node_data = this.data_mut();

        node_data.update_by_parent(parent.map(|p| p.data()), resources);

        list.inner.extend(
          node_data
            .provide_render_object()
            .into_iter()
            .map(|&render_object| Drawcall {
              render_object,
              node: this_handle,
            }),
        );
      },
    );
    list
  }
}
