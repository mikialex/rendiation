use crate::*;

use bitflags::bitflags;
use tree::*;

pub type SceneNodeData = Identity<SceneNodeDataImpl>;
pub type SceneNodeHandle = TreeNodeHandle<SceneNodeData>;

#[derive(Incremental, Clone)]
pub struct SceneNodeDataImpl {
  pub local_matrix: Mat4<f32>,
  pub visible: bool,
}

#[derive(Default, Incremental)]
pub struct SceneNodeDerivedData {
  pub world_matrix: Mat4<f32>,
  pub net_visible: bool,
}

bitflags! {
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
  pub struct SceneNodeDeriveDataDirtyFlag: u32 {
    const WorldMatrix = 0b00000001;
    const NetVisible = 0b00000010;
  }
}

impl HierarchyDirtyMark for SceneNodeDeriveDataDirtyFlag {
  fn contains(&self, mark: &Self) -> bool {
    self.contains(*mark)
  }
  fn intersects(&self, mark: &Self) -> bool {
    self.intersects(*mark)
  }
  fn insert(&mut self, mark: &Self) {
    self.insert(*mark)
  }
  fn all_dirty() -> Self {
    Self::all()
  }
}

impl IncrementalHierarchyDerived for SceneNodeDerivedData {
  type Source = SceneNodeDataImpl;

  type DirtyMark = SceneNodeDeriveDataDirtyFlag;

  fn filter_hierarchy_change(
    change: &<Self::Source as IncrementalBase>::Delta,
  ) -> Option<Self::DirtyMark> {
    match change {
      SceneNodeDataImplDelta::local_matrix(_) => SceneNodeDeriveDataDirtyFlag::WorldMatrix,
      SceneNodeDataImplDelta::visible(_) => SceneNodeDeriveDataDirtyFlag::WorldMatrix,
    }
    .into()
  }

  fn hierarchy_update(
    &mut self,
    self_source: &Self::Source,
    parent_derived: Option<&Self>,
    dirty: &Self::DirtyMark,
    mut collect: impl FnMut(Self::Delta),
  ) {
    if let Some(parent) = parent_derived {
      if dirty.intersects(SceneNodeDeriveDataDirtyFlag::WorldMatrix) {
        let new_world = parent.world_matrix * self_source.local_matrix;
        if new_world != self.world_matrix {
          self.world_matrix = new_world;
          collect(SceneNodeDerivedDataDelta::world_matrix(new_world))
        }
      }
      // too cheap, don't check flag
      let net_visible = parent.net_visible || self_source.visible;
      if net_visible != self.net_visible {
        self.net_visible = net_visible;
        collect(SceneNodeDerivedDataDelta::net_visible(net_visible))
      }
    } else {
      self.world_matrix = self_source.local_matrix;
      self.net_visible = self_source.visible;
    }
  }
}

impl Default for SceneNodeDataImpl {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::one(),
    }
  }
}

#[derive(Clone)]
pub struct SceneNode {
  inner: ShareTreeNode<ReactiveTreeCollection<SceneNodeData, SceneNodeDataImpl>>,
}

clone_self_incremental!(SceneNode);

impl SceneNode {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(Partial<SceneNodeDataImpl>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl futures::Stream<Item = U> {
    self.visit(|node| node.listen_by(mapper))
  }

  pub fn from_root(
    nodes: SharedTreeCollection<ReactiveTreeCollection<SceneNodeData, SceneNodeDataImpl>>,
  ) -> Self {
    Self {
      inner: nodes.create_new_root(Default::default()),
    }
  }

  pub fn id(&self) -> usize {
    self.inner.visit(|n| n.id())
  }

  pub fn raw_handle(&self) -> SceneNodeHandle {
    self.inner.raw_handle()
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    Self {
      inner: self.inner.create_child_default(),
    }
  }

  pub fn mutate<F: FnOnce(Mutating<SceneNodeDataImpl>) -> T, T>(&self, f: F) -> T {
    let source = self.inner.visit_raw_storage(|tree| tree.source.clone());
    let index = self.inner.raw_handle().index();
    self.inner.mutate(|node| {
      node.mutate_with(f, |delta| {
        source.emit(&tree::TreeMutation::Mutate { node: index, delta })
      })
    })
  }

  pub fn visit<F: FnOnce(&SceneNodeData) -> T, T>(&self, f: F) -> T {
    self.inner.visit(f)
  }

  pub fn visit_parent<F: FnOnce(&SceneNodeData) -> T, T>(&self, f: F) -> Option<T> {
    self.inner.visit_parent(f)
  }

  pub fn set_local_matrix(&self, mat: Mat4<f32>) {
    self.mutate(|mut node| node.modify(SceneNodeDataImplDelta::local_matrix(mat)));
  }
  pub fn get_local_matrix(&self) -> Mat4<f32> {
    self.visit(|node| node.local_matrix)
  }

  pub fn set_visible(&self, visible: bool) {
    self.mutate(|mut node| node.modify(SceneNodeDataImplDelta::visible(visible)));
  }
}
