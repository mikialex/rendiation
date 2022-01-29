use std::{
  collections::HashSet,
  hash::{Hash, Hasher},
};

use crate::*;

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug, Eq)]
pub struct ShaderFunctionMetaInfo {
  pub function_name: &'static str,
  pub function_source: Option<&'static str>, // None is builtin function, no need to gen code
  pub depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
}

impl ShaderFunctionMetaInfo {
  #[must_use]
  pub fn declare_function_dep(mut self, f: &'static ShaderFunctionMetaInfo) -> Self {
    self.depend_functions.insert(f);
    self
  }
}

impl Hash for ShaderFunctionMetaInfo {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.function_name.hash(state);
  }
}

impl PartialEq for ShaderFunctionMetaInfo {
  fn eq(&self, other: &Self) -> bool {
    self.function_name == other.function_name
  }
}

impl ShaderFunctionMetaInfo {
  pub fn new(function_name: &'static str, function_source: Option<&'static str>) -> Self {
    Self {
      function_name,
      function_source,
      depend_functions: HashSet::new(),
    }
  }
}

/// use for compile time ubo field reflection by procedure macro;
pub struct ShaderStructMetaInfo {
  pub name: &'static str,
  pub fields: Vec<(&'static str, ShaderStructMemberValueType)>,
}

impl ShaderStructMetaInfo {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      fields: Default::default(),
    }
  }

  #[must_use]
  pub fn add_field<T: ShaderStructMemberValueNodeType>(mut self, name: &'static str) -> Self {
    self.fields.push((name, T::to_type()));
    self
  }
}
