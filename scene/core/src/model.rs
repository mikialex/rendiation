use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

pub type MeshModel<Me, Ma> = SceneItemRef<MeshModelImpl<Me, Ma>>;

pub struct MeshModelImpl<Me, Ma> {
  pub material: SceneItemRef<Ma>,
  pub mesh: SceneItemRef<Me>,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl<Me, Ma> MeshModelImpl<Me, Ma> {
  pub fn new(material: Ma, mesh: Me, node: SceneNode) -> Self {
    Self {
      material: material.into(),
      mesh: mesh.into(),
      group: Default::default(),
      node,
    }
  }
}

// pub enum MeshModelDelta{
//   Material()
// }
