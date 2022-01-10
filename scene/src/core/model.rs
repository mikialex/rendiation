use std::{cell::RefCell, rc::Rc};

use rendiation_renderable_mesh::group::MeshDrawGroup;

use crate::*;

#[derive(Clone)]
pub struct MeshModel {
  pub inner: Rc<RefCell<MeshModelImpl>>,
}

impl MeshModel {
  // todo add type constraint
  pub fn new<Ma: WebGPUMaterial + 'static, Me: WebGPUMesh + 'static>(
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

pub struct MeshModelImpl<Me = Box<dyn WebGPUMesh>, Ma = Box<dyn WebGPUMaterial>> {
  pub material: Ma,
  pub mesh: Me,
  pub group: MeshDrawGroup,
  pub node: SceneNode,
}

impl MeshModelImpl {
  // todo add type constraint
  pub fn new<Ma: WebGPUMaterial + 'static, Me: WebGPUMesh + 'static>(
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

impl<Ma: WebGPUMaterial + 'static, Me: WebGPUMesh + 'static> MeshModelImpl<Me, Ma> {
  // todo add type constraint
  pub fn new_typed(material: Ma, mesh: Me, node: SceneNode) -> Self {
    Self {
      material,
      mesh,
      group: Default::default(),
      node,
    }
  }
}
