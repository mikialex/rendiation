#[derive(Default)]
pub struct BindGroupMetaInfo {
  name: String,
  entries: Vec<(wgpu::BindGroupLayoutEntry, String)>,
}

impl BindGroupMetaInfo {
  pub fn entry(
    &mut self,
    shader_name: impl Into<String>,
    entry: wgpu::BindGroupLayoutEntry,
  ) -> &mut Self {
    self.entries.push((entry, shader_name.into()));
    self
  }

  pub fn generate_wgsl(&self) -> String {
    todo!()
  }

  pub fn create_layout(&self) {
    //
  }

  pub fn create_bindgroup(&self) {
    //
  }
}
