use std::{cell::RefCell, rc::Rc};

use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[derive(Clone)]
pub struct MeshModel<Me, Ma> {
  pub inner: Rc<RefCell<MeshModelImpl<Me, Ma>>>,
}

impl<Ma: MaterialCPUResource + 'static, Me: WebGPUMesh + 'static> MeshModel<Me, Ma> {
  // todo add type constraint
  pub fn new(material: Ma, mesh: Me, node: SceneNode) -> Self {
    let inner = MeshModelImpl::new(material, mesh, node);
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

pub struct MeshModelImpl<Me, Ma> {
  pub material: MaterialInner<Ma>,
  pub mesh: Me,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl<Me, Ma> MeshModelImpl<Me, Ma> {
  // todo add type constraint
  pub fn new(material: Ma, mesh: Me, node: SceneNode) -> Self {
    Self {
      material: MaterialInner::new(material),
      mesh,
      group: Default::default(),
      node,
    }
  }
}
