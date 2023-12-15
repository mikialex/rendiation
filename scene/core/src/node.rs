use std::ops::Deref;

use bitflags::bitflags;
use tree::*;

use crate::*;

pub type SceneNodeHandle = TreeNodeHandle<SceneNodeData>;

#[derive(Incremental, Clone)]
pub struct SceneNodeDataImpl {
  pub local_matrix: Mat4<f32>,
  pub visible: bool,
}

pub struct SceneNodeData {
  guid: u64,
  pub data: SceneNodeDataImpl,
}
impl Deref for SceneNodeData {
  type Target = SceneNodeDataImpl;

  fn deref(&self) -> &Self::Target {
    &self.data
  }
}

impl SceneNodeData {
  pub fn guid(&self) -> u64 {
    self.guid
  }
}

impl Default for SceneNodeData {
  fn default() -> Self {
    Self {
      guid: alloc_global_res_id(),
      data: Default::default(),
    }
  }
}
impl Clone for SceneNodeData {
  fn clone(&self) -> Self {
    Self {
      guid: alloc_global_res_id(),
      data: self.data.clone(),
    }
  }
}

impl SimpleIncremental for SceneNodeData {
  type Delta = <SceneNodeDataImpl as IncrementalBase>::Delta;

  fn s_apply(&mut self, delta: Self::Delta) {
    self.data.apply(delta).unwrap()
  }

  fn s_expand(&self, cb: impl FnMut(Self::Delta)) {
    self.data.expand(cb)
  }
}

#[derive(Incremental, Clone)]
pub struct SceneNodeDerivedData {
  pub world_matrix: Mat4<f32>,
  pub world_matrix_inverse: Mat4<f32>,
  pub net_visible: bool,
}

impl ReversibleIncremental for SceneNodeDerivedData {
  fn reverse_delta(&self, delta: &Self::Delta) -> Self::Delta {
    use SceneNodeDerivedDataDelta as D;
    match delta {
      D::world_matrix(_) => D::world_matrix(self.world_matrix),
      D::world_matrix_inverse(_) => D::world_matrix_inverse(self.world_matrix_inverse),
      D::net_visible(_) => D::net_visible(self.net_visible),
    }
  }
}

impl HierarchyDerived for SceneNodeDerivedData {
  type Source = SceneNodeData;

  fn compute_hierarchy(self_source: &Self::Source, parent_derived: Option<&Self>) -> Self {
    if let Some(parent) = parent_derived {
      let world_matrix = parent.world_matrix * self_source.local_matrix;
      let world_matrix_inverse = world_matrix.inverse_or_identity();
      Self {
        world_matrix,
        world_matrix_inverse,
        net_visible: parent.net_visible && self_source.visible,
      }
    } else {
      SceneNodeDerivedData::build_default(self_source)
    }
  }
}

bitflags! {
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
  pub struct SceneNodeDeriveDataDirtyFlag: u8 {
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

impl HierarchyDerivedBase for SceneNodeDerivedData {
  type Source = SceneNodeData;
  fn build_default(self_source: &Self::Source) -> Self {
    SceneNodeDerivedData {
      world_matrix: self_source.data.local_matrix,
      world_matrix_inverse: self_source.data.local_matrix.inverse_or_identity(),
      net_visible: self_source.data.visible,
    }
  }
}

impl IncrementalHierarchyDerived for SceneNodeDerivedData {
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
    mut collect: impl FnMut(&mut Self, Self::Delta),
  ) {
    if let Some(parent) = parent_derived {
      if dirty.intersects(SceneNodeDeriveDataDirtyFlag::WorldMatrix) {
        let new_world = parent.world_matrix * self_source.local_matrix;
        if new_world != self.world_matrix {
          self.world_matrix = new_world;
          self.world_matrix_inverse = new_world.inverse_or_identity();
          collect(self, SceneNodeDerivedDataDelta::world_matrix(new_world));
          collect(
            self,
            SceneNodeDerivedDataDelta::world_matrix_inverse(self.world_matrix_inverse),
          )
        }
      }
      // too cheap, don't check flag
      let net_visible = parent.net_visible || self_source.visible;
      if net_visible != self.net_visible {
        self.net_visible = net_visible;
        collect(self, SceneNodeDerivedDataDelta::net_visible(net_visible))
      }
    } else {
      let new_world = self_source.local_matrix;
      if new_world != self.world_matrix {
        self.world_matrix = new_world;
        collect(self, SceneNodeDerivedDataDelta::world_matrix(new_world))
      }
      let net_visible = self_source.visible;
      if net_visible != self.net_visible {
        self.net_visible = net_visible;
        collect(self, SceneNodeDerivedDataDelta::net_visible(net_visible))
      }
    }
  }
}

impl Default for SceneNodeDataImpl {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::identity(),
    }
  }
}

#[derive(Clone)]
pub struct SceneNode {
  pub(crate) guid: u64,
  pub(crate) scene_id: u64,
  pub(crate) inner: ShareTreeNode<
    ReactiveTreeCollection<parking_lot::RwLock<TreeCollection<SceneNodeData>>, SceneNodeData>,
  >,
}

clone_self_incremental!(SceneNode);

impl GlobalIdentified for SceneNode {
  fn guid(&self) -> u64 {
    self.guid
  }
}

/// (scene guid, alloc idx)
pub type NodeIdentity = (u64, usize);

impl SceneNode {
  pub fn scene_and_node_id(&self) -> NodeIdentity {
    (self.scene_id, self.inner.raw_handle().index())
  }

  pub(crate) fn create_new(
    nodes: SceneNodeCollectionImpl,
    data: SceneNodeData,
    scene_id: u64,
  ) -> Self {
    Self {
      guid: data.guid,
      scene_id,
      inner: ShareTreeNode::new_as_root(data, &nodes),
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

  pub fn detach_from_parent(&self) -> Result<(), TreeMutationError> {
    self.inner.detach_from_parent()
  }

  pub fn attach_to(&self, parent: &Self) -> Result<(), TreeMutationError> {
    self.inner.attach_to(&parent.inner)
  }

  #[must_use]
  pub fn create_child(&self) -> Self {
    let inner = self.inner.create_child_default();
    let guid = alloc_global_res_id();
    let scene_id = self.scene_id;
    Self {
      inner,
      guid,
      scene_id,
    }
  }

  pub fn mutate<F: FnOnce(Mutating<SceneNodeData>) -> T, T>(&self, f: F) -> T {
    let source = self.inner.visit_raw_storage(|tree| tree.source.clone());
    let index = self.inner.raw_handle().index();
    self.inner.mutate(|node| {
      f(Mutating::new(node, &mut |delta, _| {
        source.emit(&tree::TreeMutation::Mutate {
          node: index,
          delta: delta.clone(),
        })
      }))
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
