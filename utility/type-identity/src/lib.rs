#![feature(const_type_name)]

use std::{
  any::{Any, TypeId},
  hash::{Hash, Hasher},
  ops::{Deref, DerefMut},
};

/// This trait is to workaround the limitation that Any only implemented for static types
pub trait TypeIdentityHash {
  fn hash_type_identity(&self, _hasher: &mut dyn Hasher);
}

#[repr(transparent)]
pub struct TypeHashProvideByTypeName<T>(pub T);

impl<T> Deref for TypeHashProvideByTypeName<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for TypeHashProvideByTypeName<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> TypeIdentityHash for TypeHashProvideByTypeName<T> {
  fn hash_type_identity(&self, mut hasher: &mut dyn Hasher) {
    if is_type_name_too_long::<T>() {
      println!(
        "warning: type name too long: {}",
        std::any::type_name::<T>()
      )
    }
    std::any::type_name::<T>().hash(&mut (hasher));
  }
}

/// util to check type name in const fn, too long type name is bad for hashing performance, and in
/// some case it's possible for user to construct super long type name and we should emit compile
/// warnings for this case
const fn is_type_name_too_long<T>() -> bool {
  std::any::type_name::<T>().len() > 64
}

#[repr(transparent)]
pub struct TypeHashProvideByTypeId<T>(pub T);

impl<T> Deref for TypeHashProvideByTypeId<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> DerefMut for TypeHashProvideByTypeId<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> TypeIdentityHash for TypeHashProvideByTypeId<T>
where
  T: Any,
{
  fn hash_type_identity(&self, mut hasher: &mut dyn Hasher) {
    TypeId::of::<T>().hash(&mut (hasher));
  }
}

pub trait TypeHashWrapperExt: Sized {
  fn type_hash_by_type_id(self) -> TypeHashProvideByTypeId<Self>
  where
    Self: Any;
  fn type_hash_by_type_name(self) -> TypeHashProvideByTypeName<Self>;
}

impl<T> TypeHashWrapperExt for T {
  fn type_hash_by_type_id(self) -> TypeHashProvideByTypeId<Self>
  where
    Self: Any,
  {
    TypeHashProvideByTypeId(self)
  }

  fn type_hash_by_type_name(self) -> TypeHashProvideByTypeName<Self> {
    TypeHashProvideByTypeName(self)
  }
}
