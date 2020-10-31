use crate::ShaderGraphNodeType;
use std::collections::HashMap;

/// use for compile time ubo field reflection by procedure macro;
pub struct UBOInfo {
  pub name: &'static str,
  pub fields: HashMap<&'static str, &'static str>, // fields name -> shader type name
  pub fields_record: Vec<&'static str>,
  pub code_cache: String,
}

impl UBOInfo {
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
      + &self.name
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
