use rendiation_algebra::Vec3;
use rendiation_algebra::Vector;

use crate::renderer::Renderable;

use super::SceneResource;

pub trait Background: 'static + Renderable<Resource = SceneResource> {
  fn require_pass_clear(&self) -> Option<wgpu::Color>;
}

pub struct SolidBackground {
  pub intensity: Vec3<f32>,
}

impl Renderable for SolidBackground {
  type Resource = SceneResource;

  fn update(
    &mut self,
    renderer: &crate::renderer::Renderer,
    res: &mut Self::Resource,
    encoder: &mut wgpu::CommandEncoder,
  ) {
  }

  fn render<'a>(&mut self, pass: &mut wgpu::RenderPass<'a>, res: &'a Self::Resource) {}
}

impl Background for SolidBackground {
  fn require_pass_clear(&self) -> Option<wgpu::Color> {
    wgpu::Color {
      r: self.intensity.r() as f64,
      g: self.intensity.g() as f64,
      b: self.intensity.b() as f64,
      a: 1.,
    }
    .into()
  }
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      intensity: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl SolidBackground {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

pub struct GradientBackground {
  pub top_intensity: Vec3<f32>,
  pub bottom_intensity: Vec3<f32>,
}
