use bitflags::bitflags;
use reactive::DefaultUnboundChannel;
use tree::*;

use crate::*;

pub type SceneNodeData = Identity<SceneNodeDataImpl>;
pub type SceneNodeHandle = TreeNodeHandle<SceneNodeData>;

#[derive(Incremental, Clone)]
pub struct SceneNodeDataImpl {
  pub local_matrix: Mat4<f32>,
  pub visible: bool,
}

#[derive(Default, Incremental, Clone)]
pub struct SceneNodeDerivedData {
  pub world_matrix: Mat4<f32>,
  pub net_visible: bool,
}

impl HierarchyDerived for SceneNodeDerivedData {
  type Source = SceneNodeDataImpl;

  fn compute_hierarchy(self_source: &Self::Source, parent_derived: Option<&Self>) -> Self {
    if let Some(parent) = parent_derived {
      Self {
        world_matrix: parent.world_matrix * self_source.local_matrix,
        net_visible: parent.net_visible && self_source.visible,
      }
    } else {
      Default::default()
    }
  }
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
      let new_world = self_source.local_matrix;
      if new_world != self.world_matrix {
        self.world_matrix = new_world;
        collect(SceneNodeDerivedDataDelta::world_matrix(new_world))
      }
      let net_visible = self_source.visible;
      if net_visible != self.net_visible {
        self.net_visible = net_visible;
        collect(SceneNodeDerivedDataDelta::net_visible(net_visible))
      }
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
  pub(crate) guid: usize,
  pub(crate) scene_id: usize,
  pub(crate) inner:
    ShareTreeNode<ReactiveTreeCollection<RwLock<TreeCollection<SceneNodeData>>, SceneNodeDataImpl>>,
}

clone_self_incremental!(SceneNode);

impl GlobalIdentified for SceneNode {
  fn guid(&self) -> usize {
    self.guid
  }
}

impl SceneNode {
  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(MaybeDeltaRef<SceneNodeDataImpl>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U> {
    self.visit(|node| node.listen_by::<DefaultUnboundChannel, _>(mapper))
  }

  pub(crate) fn create_new(
    nodes: SceneNodeCollectionInner,
    data: SceneNodeDataImpl,
    scene_id: usize,
  ) -> Self {
    let identity = Identity::new(data);
    Self {
      guid: identity.guid(),
      scene_id,
      inner: nodes.create_new_root(identity),
    }
  }

  pub fn get_node_collection(&self) -> SceneNodeCollection {
    SceneNodeCollection {
      inner: self.inner.get_node_collection(),
      scene_guid: self.scene_id,
    }
  }

  pub fn raw_handle(&self) -> SceneNodeHandle {
    self.inner.raw_handle()
  }

  pub fn raw_handle_parent(&self) -> Option<SceneNodeHandle> {
    self.inner.raw_handle_parent()
  }

  pub fn detach_from_parent(&self) {
    self.inner.detach_from_parent()
  }

  pub fn attach_to(&self, parent: &Self) {
    self.inner.attach_to(&parent.inner)
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    let inner = self.inner.create_child_default();
    let guid = inner.visit(|n| n.guid());
    let scene_id = self.scene_id;
    Self {
      inner,
      guid,
      scene_id,
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
