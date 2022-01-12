use std::{cell::RefCell, rc::Rc};

use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

pub struct MeshModel<Me, Ma> {
  pub inner: Rc<RefCell<ResourceWrapped<MeshModelImpl<Me, Ma>>>>,
}

impl<Me, Ma> Clone for MeshModel<Me, Ma> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<Ma, Me> MeshModel<Me, Ma> {
  pub fn new(material: Ma, mesh: Me, node: SceneNode) -> Self {
    let inner = MeshModelImpl::new(material, mesh, node);
    Self {
      inner: Rc::new(RefCell::new(inner.into_resourced())),
    }
  }
}

pub struct MeshModelImpl<Me, Ma> {
  pub material: Ma,
  pub mesh: Me,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl<Me, Ma> MeshModelImpl<Me, Ma> {
  // todo add type constraint
  pub fn new(material: Ma, mesh: Me, node: SceneNode) -> Self {
    Self {
      material,
      mesh,
      group: Default::default(),
      node,
    }
  }
}
