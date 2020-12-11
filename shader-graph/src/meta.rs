use std::{
  collections::{HashMap, HashSet},
  hash::{Hash, Hasher},
};

use crate::ShaderGraphNodeType;

/// use for compile time ubo field reflection by procedure macro;
#[derive(Debug, Eq)]
pub struct ShaderFunctionMetaInfo {
  pub function_name: &'static str,
  pub function_source: Option<&'static str>, // None is builtin function, no need to gen code
  pub depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
}

impl ShaderFunctionMetaInfo {
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
pub struct UBOMetaInfo {
  pub name: &'static str,
  pub fields: HashMap<&'static str, &'static str>, // fields name -> shader type name
  pub fields_record: Vec<&'static str>,
  pub code_cache: String,
}

impl UBOMetaInfo {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      fields: HashMap::new(),
      fields_record: Vec::new(),
      code_cache: String::new(),
    }
  }
  pub fn add_field<T: ShaderGraphNodeType>(mut self, name: &'static str) -> Self {
    self.fields.insert(name, T::to_glsl_type());
    self.fields_record.push(name);
    self
  }

  pub fn gen_code_cache(mut self) -> Self {
    self.code_cache = String::from("uniform ")
      + self.name
      + " {\n"
      + self
        .fields_record
        .iter()
        .map(|&s| (s, *self.fields.get(s).unwrap()))
        .map(|(name, ty)| format!("  {} {}", ty, name))
        .collect::<Vec<_>>()
        .join(";\n")
        .as_str()
      + ";"
      + " \n}";

    self
  }
}
