use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

pub type SceneModel = SceneItemRef<SceneModelImpl>;

#[derive(Incremental)]
pub struct SceneModelImpl {
  pub model: ModelType,
  pub node: SceneNode,
}

#[non_exhaustive]
#[derive(Clone)]
pub enum ModelType {
  Standard(SceneItemRef<StandardModel>),
  Foreign(Box<dyn AnyClone + Send + Sync>),
}

clone_self_incremental!(ModelType);

impl ModelType {
  pub fn guid(&self) -> Option<usize> {
    match self {
      Self::Standard(m) => m.guid(),
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(GlobalIdentified)
        .downcast_ref(m.as_ref().as_any())?
        .guid(),
    }
    .into()
  }
}

#[derive(Incremental)]
pub struct StandardModel {
  pub material: SceneMaterialType,
  pub mesh: SceneMeshType,
  pub group: MeshDrawGroup,
  pub skeleton: Option<Skeleton>,
}

impl StandardModel {
  pub fn new(material: impl Into<SceneMaterialType>, mesh: impl Into<SceneMeshType>) -> Self {
    Self {
      material: material.into(),
      mesh: mesh.into(),
      group: Default::default(),
      skeleton: Default::default(),
    }
  }
}

pub type Skeleton = SceneItemRef<SkeletonImpl>;
#[derive(Clone)]
pub struct SkeletonImpl {
  pub joints: Vec<Joint>,
}
clone_self_incremental!(SkeletonImpl);

impl SkeletonImpl {
  /// recover the skeleton to bind-time pose
  pub fn pose(&self) {
    // todo improve, cache compute
    let id_map = self
      .joints
      .iter()
      .enumerate()
      .map(|(index, joint)| (joint.node.guid(), index))
      .collect::<FastHashMap<_, _>>();

    self.joints.iter().for_each(|joint| {
      let bone_local = if let Some(parent_id) = joint.node.visit_parent(|p| p.guid())
        && let Some(parent_index) = id_map.get(&parent_id) {
        let parent_bind_inverse = &self.joints[*parent_index].bind_inverse;
        *parent_bind_inverse * joint.bind_inverse.inverse_or_identity()
      } else {
        joint.bind_inverse.inverse_or_identity()
      };
      joint.node.set_local_matrix(bone_local)
    })
  }
}

#[derive(Clone)]
pub struct Joint {
  pub node: SceneNode,
  /// the transformation from the local-space to joint-space
  /// local -> joint is like world -> local
  pub bind_inverse: Mat4<f32>,
}

impl Joint {
  /// we do binding in the model's joint-space. that's why we need bind_inverse matrix;
  /// so, we should first: from local to joint-space: apply bind_inverse
  /// then, we apply the real skeleton matrix, to express the correct new skinned-local-space
  pub fn compute_offset_matrix(&self, d_sys: &SceneNodeDeriveSystem) -> Mat4<f32> {
    d_sys.get_world_matrix(&self.node) * self.bind_inverse
  }
}
