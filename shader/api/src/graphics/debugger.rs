pub struct ShaderBuilderDebugger {
  scopes: Vec<ShaderBuildDebugScope>,
}

#[derive(Debug)]
pub struct ShaderBuildDebugScope {
  pub name: String,
  pub items: Vec<ShaderBuildingDebugItem>,
}

#[derive(Debug)]
pub enum ShaderBuildingDebugItem {
  InjectSemantic(String),
  UseSemantic(String),
  SubScope(ShaderBuildDebugScope),
}

impl ShaderBuilderDebugger {
  pub fn inject(&mut self, label: String) {
    self
      .scopes
      .last_mut()
      .unwrap()
      .items
      .push(ShaderBuildingDebugItem::InjectSemantic(label));
  }

  pub fn use_semantic(&mut self, label: String) {
    self
      .scopes
      .last_mut()
      .unwrap()
      .items
      .push(ShaderBuildingDebugItem::UseSemantic(label));
  }

  pub fn new_scope(&mut self, label: String) {
    self.scopes.push(ShaderBuildDebugScope {
      name: label,
      items: vec![],
    });
  }

  pub fn close_scope(&mut self) {
    let top = self.scopes.pop().unwrap();
    self
      .scopes
      .last_mut()
      .unwrap()
      .items
      .push(ShaderBuildingDebugItem::SubScope(top));
  }
}

impl Default for ShaderBuilderDebugger {
  fn default() -> Self {
    Self {
      scopes: vec![ShaderBuildDebugScope {
        name: "root".to_string(),
        items: vec![],
      }],
    }
  }
}
