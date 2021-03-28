use crate::ComponentHandle;

pub trait ShaderComponent {
  fn build(&self) {}
  fn shader_key(&self) {}
}

pub struct Material {
  pub components: Vec<ComponentHandle>,
}

pub trait Light {}
