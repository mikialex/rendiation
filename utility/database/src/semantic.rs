use crate::*;

pub trait EntitySemantic: Any + Send + Sync {
  fn entity_id() -> EntityId {
    EntityId(TypeId::of::<Self>())
  }
  fn display_name() -> &'static str {
    std::any::type_name::<Self>()
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub TypeId);

pub trait EntityAssociateSemantic: Any + Send + Sync {
  type Entity: EntitySemantic;
  fn display_name() -> &'static str {
    std::any::type_name::<Self>()
  }
}

/// Statically associate entity semantic, component semantic and component type
pub trait ComponentSemantic: EntityAssociateSemantic {
  type Data: DataBaseDataType;

  /// Even if the Data has Default bound, user may still define custom default value
  /// for each different component type.
  fn default_override() -> Self::Data {
    Self::Data::default()
  }

  fn component_id() -> ComponentId {
    ComponentId(TypeId::of::<Self>())
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub TypeId);

pub type ForeignKeyComponentData = Option<RawEntityHandle>;
pub trait ForeignKeySemantic: ComponentSemantic<Data = ForeignKeyComponentData> {
  type ForeignEntity: EntitySemantic;
}

#[macro_export]
macro_rules! declare_entity {
  ($Type: tt) => {
    pub struct $Type;
    impl EntitySemantic for $Type {
      fn display_name() -> &'static str {
        stringify!($Type)
      }
    }
  };
}

#[macro_export]
macro_rules! declare_entity_associated {
  ($Type: tt, $EntityTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
      fn display_name() -> &'static str {
        stringify!($Type)
      }
    }
  };
}

#[macro_export]
macro_rules! declare_component {
  ($Type: tt, $EntityTy: ty, $DataTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
      fn display_name() -> &'static str {
        stringify!($Type)
      }
    }
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
    }
  };

  ($Type: tt, $EntityTy: ty, $DataTy: ty, $DefaultOverride: expr) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
      fn display_name() -> &'static str {
        stringify!($Type)
      }
    }
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
      fn default_override() -> Self::Data {
        $DefaultOverride
      }
    }
  };
}

#[macro_export]
macro_rules! declare_foreign_key {
  ($Type: tt,  $EntityTy: ty, $ForeignEntityTy: ty) => {
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
      fn display_name() -> &'static str {
        stringify!($Type)
      }
    }
    impl ComponentSemantic for $Type {
      type Data = ForeignKeyComponentData;
    }
    impl ForeignKeySemantic for $Type {
      type ForeignEntity = $ForeignEntityTy;
    }
  };
}
