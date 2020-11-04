use std::{
  collections::HashSet,
  hash::{Hash, Hasher},
};

pub struct ShaderBuiltInFunction {
  pub function_name: &'static str,
}

#[derive(Debug, Eq)]
pub struct ShaderFunction {
  pub function_name: &'static str,
  pub function_source: Option<&'static str>, // None is builtin function, no need to gen code
  pub depend_functions: HashSet<&'static ShaderFunction>,
}

impl ShaderFunction {
  pub fn declare_function_dep(mut self, f: &'static ShaderFunction) -> Self {
    self.depend_functions.insert(f);
    self
  }
}

impl Hash for ShaderFunction {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.function_name.hash(state);
  }
}

impl PartialEq for ShaderFunction {
  fn eq(&self, other: &Self) -> bool {
    self.function_name == other.function_name
  }
}

impl ShaderFunction {
  pub fn new(function_name: &'static str, function_source: Option<&'static str>) -> Self {
    Self {
      function_name,
      function_source,
      depend_functions: HashSet::new(),
    }
  }
}
