use crate::*;

pub trait EntitySemantic: Any + Send + Sync {
  fn entity_id() -> EntityId {
    EntityId(TypeId::of::<Self>())
  }
  /// this name must be unique in db(this will be runtime checked)
  /// this name also used in serialization, so it must be stable
  ///
  /// The default implementation will use the std::any::type_name
  /// the stability of the name is not guaranteed if you modify the typename
  ///  or move it to other module/crate
  fn unique_name() -> &'static str {
    std::any::type_name::<Self>()
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub TypeId);

pub trait EntityAssociateSemantic: Any + Send + Sync {
  type Entity: EntitySemantic;
  /// this name must be unique in db(this will be runtime checked)
  /// this name also used in serialization, so it must be stable
  ///
  /// The default implementation will use the std::any::type_name
  /// the stability of the name is not guaranteed if you modify the typename
  ///  or move it to other module/crate
  fn unique_name() -> &'static str {
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
    ComponentId::TypeId(TypeId::of::<Self>())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
  TypeId(TypeId),
  Hash(u64),
}

pub type ForeignKeyComponentData = Option<RawEntityHandle>;
pub trait ForeignKeySemantic: ComponentSemantic<Data = ForeignKeyComponentData> {
  type ForeignEntity: EntitySemantic;
}

#[macro_export]
macro_rules! declare_entity {
  ($(#[$attr:meta])* $Type: ident) => {
    #[doc = concat!(stringify!($Type), " entity.\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl EntitySemantic for $Type {}
  };
}

#[macro_export]
macro_rules! declare_entity_associated {
  ($(#[$attr:meta])* $Type: ident, $EntityTy: ty) => {
    #[doc = concat!(stringify!($Type), " is associated with ", stringify!($EntityTy), ".\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
    }
  };
}

#[macro_export]
macro_rules! declare_component {
  ($(#[$attr:meta])* $Type: ident, $EntityTy: ty, $DataTy: ty) => {
    #[doc = concat!(stringify!($Type), " is ", stringify!($EntityTy), "'s component.\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
    }
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
    }
  };

  ($(#[$attr:meta])* $Type: ident, $EntityTy: ty, $DataTy: ty, $DefaultOverride: expr) => {
    #[doc = concat!(stringify!($Type), " is ", stringify!($EntityTy), "'s component.\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl EntityAssociateSemantic for $Type {
      type Entity = $EntityTy;
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
  ($(#[$attr:meta])* $Type: ident,  $EntityTy: ty, $ForeignEntityTy: ty) => {
    #[doc = concat!(stringify!($Type), " is ", stringify!($EntityTy), "'s foreign key referencing ", stringify!($ForeignEntityTy), ".\n\n")]
    $(#[$attr])*
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
