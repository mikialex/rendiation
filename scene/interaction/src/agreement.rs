use crate::*;

pub trait PickingConventionAgreement {
  fn is_keep_mesh_shape(&self) -> bool;
}

define_dyn_trait_downcaster_static!(PickingConventionAgreement);
pub fn register_picking_agreement_feature<T>()
where
  T: AsRef<dyn PickingConventionAgreement> + AsMut<dyn PickingConventionAgreement> + 'static,
{
  get_dyn_trait_downcaster_static!(PickingConventionAgreement).register::<T>()
}

impl PickingConventionAgreement for SceneMaterialType {
  fn is_keep_mesh_shape(&self) -> bool {
    match self {
      Self::PhysicalSpecularGlossiness(m) => m.is_keep_mesh_shape(),
      Self::PhysicalMetallicRoughness(m) => m.is_keep_mesh_shape(),
      Self::Flat(m) => m.is_keep_mesh_shape(),
      Self::Foreign(m) => {
        if let Some(v) = get_dyn_trait_downcaster_static!(PickingConventionAgreement)
          .downcast_ref(m.as_ref().as_any())
        {
          v.is_keep_mesh_shape()
        } else {
          false
        }
      }
      _ => false,
    }
  }
}

impl<T> PickingConventionAgreement for SharedIncrementalSignal<T>
where
  T: PickingConventionAgreement + IncrementalBase,
{
  fn is_keep_mesh_shape(&self) -> bool {
    self.read().is_keep_mesh_shape()
  }
}

impl PickingConventionAgreement for FlatMaterial {
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
}

impl PickingConventionAgreement for PhysicalSpecularGlossinessMaterial {
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
}

impl PickingConventionAgreement for PhysicalMetallicRoughnessMaterial {
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
}
