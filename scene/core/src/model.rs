use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[non_exhaustive]
#[derive(Clone)]
pub enum SceneModelType {
  Standard(SceneItemRef<StandardModel>),
  Foreign(Arc<dyn Any + Send + Sync>),
}

clone_self_incremental!(SceneModelType);

pub type SceneModel = SceneItemRef<SceneModelImpl>;

#[derive(Incremental)]
pub struct SceneModelImpl {
  pub model: SceneModelType,
  pub node: SceneNode,
}

#[derive(Incremental)]
pub struct StandardModel {
  pub material: SceneMaterialType,
  pub mesh: SceneMeshType,
  pub group: MeshDrawGroup,
}

pub struct SkinnedModel {
  skeleton: Skeleton,
}

pub struct Skeleton {
  joints: Vec<Joint>,
}

impl Skeleton {
  /// recover the skeleton to bind-time pose
  pub fn pose(&self) {
    // todo improve, cache compute
    let id_map = self
      .joints
      .iter()
      .enumerate()
      .map(|(index, joint)| (joint.node.id(), index))
      .collect::<HashMap<_, _>>();

    self.joints.iter().for_each(|joint| {
      let bone_local = if let Some(parent_id) = joint.node.visit_parent(|p| p.id())
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

pub struct Joint {
  node: SceneNode,
  /// the transformation from the bind-space to the local-space
  bind_inverse: Mat4<f32>,
}

impl Joint {
  pub fn compute_offset_matrix(&self) -> Mat4<f32> {
    self.node.get_world_matrix() * self.bind_inverse
  }
}
