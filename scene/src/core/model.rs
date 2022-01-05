use std::{cell::RefCell, rc::Rc};

use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[derive(Clone)]
pub struct MeshModel {
  pub inner: Rc<RefCell<MeshModelImpl>>,
}

impl MeshModel {
  // todo add type constraint
  pub fn new<Ma: Material + 'static, Me: Mesh + 'static>(
    material: Ma,
    mesh: Me,
    node: SceneNode,
  ) -> Self {
    let inner = MeshModelImpl::new(material, mesh, node);
    Self {
      inner: Rc::new(RefCell::new(inner)),
    }
  }
}

pub struct MeshModelImpl<Me = Box<dyn Mesh>, Ma = Box<dyn Material>> {
  pub material: Ma,
  pub mesh: Me,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl MeshModelImpl {
  // todo add type constraint
  pub fn new<Ma: Material + 'static, Me: Mesh + 'static>(
    material: Ma,
    mesh: Me,
    node: SceneNode,
  ) -> Self {
    Self {
      material: Box::new(material),
      mesh: Box::new(mesh),
      group: Default::default(),
      node,
    }
  }
}
