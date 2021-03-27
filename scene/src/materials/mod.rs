use crate::ComponentHandle;

pub trait ShaderComponent {}

pub trait ShadingComponent {}

pub struct Material {
  components: Vec<ComponentHandle>,
}
