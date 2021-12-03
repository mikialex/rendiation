use crate::{BindableResource, BindableResourceWgslCodeGen};

#[derive(Default)]
pub struct BindGroupMetaInfo {
  name: String,
  entries: Vec<(wgpu::BindGroupLayoutEntry, String)>,
}

impl BindGroupMetaInfo {
  pub fn entry<T>(
    &mut self,
    shader_name: impl Into<String>,
    visibility: wgpu::ShaderStages,
  ) -> &mut Self
  where
    T: BindableResource + BindableResourceWgslCodeGen,
  {
    let entry = wgpu::BindGroupLayoutEntry {
      binding: self.entries.len() as u32,
      visibility,
      ty: T::bind_layout(),
      count: None,
    };

    self.entries.push((entry, shader_name.into()));
    self
  }

  pub fn generate_wgsl(&self) -> String {
    self
      .entries
      .iter()
      .map(|(entry, name)| {
        format!(
          "
      //
      "
        )
      })
      .collect::<Vec<String>>()
      .join("\n")
  }

  pub fn create_layout(&self) {
    //
  }

  pub fn create_bindgroup(&self) {
    //
  }
}
