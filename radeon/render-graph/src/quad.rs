use crate::ImmediateRenderableContent;
use rendiation_ral::*;
use rendiation_renderable_mesh::{tessellation::*, vertex::*};

pub struct FullScreenQuad<T: RAL> {
  obj: Drawcall<T>,
}

pub struct FullScreenQuadFactory<T: RAL> {
  geometry: GeometryHandle<T, Vertex>,
}

impl<T: RAL> FullScreenQuadFactory<T> {
  pub fn new(res: &mut ResourceManager<T>, renderer: &mut T::Renderer) -> Self {
    let geometry = Quad.tessellate().geometry;
    let geometry = geometry.create(res, renderer);
    let geometry = res.add_geometry(geometry);
    Self { geometry }
  }

  pub fn create_quad<SP: ShadingProvider<T, Geometry = Vertex>>(
    &self,
    shading: ShadingHandle<T, SP>,
  ) -> FullScreenQuad<T> {
    FullScreenQuad {
      obj: Drawcall::new(self.geometry, shading),
    }
  }
}

impl<T: RAL> ImmediateRenderableContent<T> for FullScreenQuad<T> {
  fn render(&self, pass: &mut T::RenderPass, res: &ResourceManager<T>) {
    T::render_drawcall(&self.obj, pass, res)
  }

  fn prepare(&mut self, renderer: &mut T::Renderer, resource: &mut ResourceManager<T>) {
    resource.maintain_gpu(renderer)
  }
}
