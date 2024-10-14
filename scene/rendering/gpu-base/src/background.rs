use crate::*;

pub struct SceneRendererRenderer {
  background: ComponentReadView<SceneSolidBackground>,
}

impl SceneRendererRenderer {
  pub fn new_from_global() -> Self {
    Self {
      background: global_entity_component_of::<SceneSolidBackground>().read(),
    }
  }

  pub fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    let color = self.background.get_value(scene).unwrap();
    let color = color.unwrap_or(Vec3::splat(0.9));
    let color = rendiation_webgpu::Color {
      r: color.x as f64,
      g: color.y as f64,
      b: color.z as f64,
      a: 1.,
    };
    (clear(color), clear(1.))
  }
}
