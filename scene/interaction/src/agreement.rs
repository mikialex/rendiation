use crate::*;

pub trait PickingConventionAgreement {
  /// decide if the material's display affect static mesh shape.
  /// most of material do not have vertex shader logic so default to true.
  fn is_keep_mesh_shape(&self) -> bool {
    true
  }
}

define_dyn_trait_downcaster_static!(PickingConventionAgreement);
pub fn register_picking_agreement_feature<T>()
where
  T: AsRef<dyn PickingConventionAgreement> + AsMut<dyn PickingConventionAgreement> + 'static,
{
  get_dyn_trait_downcaster_static!(PickingConventionAgreement).register::<T>()
}

impl PickingConventionAgreement for MaterialEnum {
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
    }
  }
}

impl<T> PickingConventionAgreement for IncrementalSignalPtr<T>
where
  T: PickingConventionAgreement + IncrementalBase,
{
  fn is_keep_mesh_shape(&self) -> bool {
    self.read().is_keep_mesh_shape()
  }
}

impl PickingConventionAgreement for FlatMaterial {}
impl PickingConventionAgreement for PhysicalSpecularGlossinessMaterial {}
impl PickingConventionAgreement for PhysicalMetallicRoughnessMaterial {}
