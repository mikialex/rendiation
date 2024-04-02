use crate::*;

pub trait EntitySemantic: Any + Send + Sync {
  fn entity_id() -> EntityId {
    EntityId(TypeId::of::<Self>())
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub TypeId);

pub trait EntityAssociateSemantic: Any + Send + Sync {
  type Entity: EntitySemantic;
}

/// Statically associate entity semantic, component semantic and component type
pub trait ComponentSemantic: EntityAssociateSemantic {
  type Data: CValue + Default;
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
macro_rules! declare_entity_associated {
  ($Type: tt, $EntityTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
    }
  };
}

#[macro_export]
macro_rules! declare_component {
  ($Type: tt, $EntityTy: ty, $DataTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
    }
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
    }
  };
}

#[macro_export]
macro_rules! declare_foreign_key {
  ($Type: tt,  $EntityTy: ty, $ForeignEntityTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
    }
    impl ComponentSemantic for $Type {
      type Data = ForeignKeyComponentData;
    }
    impl ForeignKeySemantic for $Type {
      type ForeignEntity = $ForeignEntityTy;
    }
  };
}
