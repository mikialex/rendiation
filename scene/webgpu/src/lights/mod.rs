pub mod directional;
pub use directional::*;
use webgpu::ShaderUniformBlock;

pub struct LightList<T> {
  pub lights: Vec<T>,
}

pub static LIGHT_TRANSMISSION_MODEL: &str = "
  struct IncidentLight {
    vec3 color;
    vec3 direction;
  };
  
  struct ReflectedLight {
    vec3 directDiffuse;
    vec3 directSpecular;
    vec3 indirectDiffuse;
    vec3 indirectSpecular;
  };
  
  struct GeometricContext {
    vec3 position;
    vec3 normal;
    vec3 viewDir;
  };
  ";

pub trait ShaderLight: ShaderUniformBlock {
  fn name() -> &'static str;
}

pub trait DirectShaderLight: ShaderLight {
  fn compute_direct_light() -> &'static str;
}
