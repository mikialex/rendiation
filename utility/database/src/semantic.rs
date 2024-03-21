use crate::*;

/// Statically associate entity semantic, component semantic and component type
pub trait ComponentSemantic: Any + Send + Sync {
  type Data: CValue + Default;
  type Entity: Any;
}

pub type ForeignKeyComponentData = Option<u32>;
pub trait ForeignKeySemantic: ComponentSemantic<Data = ForeignKeyComponentData> {
  type ForeignEntity: Any;
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
    impl ComponentSemantic for $Type {
      type Data = ForeignKeyComponentData;
      type Entity = $EntityTy;
    }
    impl ForeignKeySemantic for $Type {
      type ForeignEntity = $ForeignEntityTy;
    }
  };
}
