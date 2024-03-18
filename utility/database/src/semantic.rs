use crate::*;

/// Statically associate entity semantic, component semantic and component type
pub trait ComponentSemantic: Any {
  type Data: CValue + Default;
  type Entity: Any;
}

pub trait ForeignKeySemantic: Any {
  type ForeignEntity: Any;
  type Entity: Any;
}

#[macro_export]
macro_rules! declare_component {
  ($Type: tt, $EntityTy: ty, $DataTy: ty) => {
    pub struct $Type;
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
      type Entity = $EntityTy;
    }
  };
}

#[macro_export]
macro_rules! declare_foreign_key {
  ($Type: tt,  $EntityTy: ty, $ForeignEntityTy: ty) => {
    pub struct $Type;
    impl ForeignKeySemantic for $Type {
      type ForeignEntity = $ForeignEntityTy;
      type Entity = $EntityTy;
    }
  };
}
