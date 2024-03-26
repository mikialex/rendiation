use crate::*;

pub trait EntitySemantic: Any + Send + Sync {
  fn entity_id() -> EntityId {
    EntityId(TypeId::of::<Self>())
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub TypeId);

/// Statically associate entity semantic, component semantic and component type
pub trait ComponentSemantic: Any + Send + Sync {
  type Data: CValue + Default;
  type Entity: EntitySemantic;
  fn component_id() -> ComponentId {
    ComponentId(TypeId::of::<Self>())
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub TypeId);

pub type ForeignKeyComponentData = Option<u32>;
pub trait ForeignKeySemantic: ComponentSemantic<Data = ForeignKeyComponentData> {
  type ForeignEntity: EntitySemantic;
}

#[macro_export]
macro_rules! declare_entity {
  ($Type: tt) => {
    pub struct $Type;
    impl EntitySemantic for $Type {}
  };
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
