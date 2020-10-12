use crate::ImmediateRenderableContent;
use rendiation_mesh_buffer::tessellation::{plane::Quad, IndexedBufferTessellator};
use rendiation_ral::{GeometryHandle, RALBackend, RenderObject, ResourceManager, ShadingProvider};

pub struct FullScreenQuad<T: RALBackend, SP: ShadingProvider<T>> {
  obj: RenderObject<T, SP>,
}

pub struct FullScreenQuadFactory<T: RALBackend> {
  geometry: GeometryHandle<T>,
}

impl<T: RALBackend> FullScreenQuadFactory<T> {
  pub fn new(res: &mut ResourceManager<T>, renderer: T::Renderer) -> Self {
    let geometry = Quad.create_mesh(&());
    todo!()
  }

  pub fn create_quad<SP: ShadingProvider<T>>(
    res: &mut ResourceManager<T>,
    // s: SP
  ) -> FullScreenQuad<T, SP> {
    todo!()
  }
}

impl<T: RALBackend, SP: ShadingProvider<T>> FullScreenQuad<T, SP> {
  pub fn new() -> Self {
    todo!()
  }
}

impl<T: RALBackend, SP: ShadingProvider<T>> ImmediateRenderableContent<T>
  for FullScreenQuad<T, SP>
{
  fn render(&self, pass: &mut T::RenderPass, res: &ResourceManager<T>) {
    todo!()
  }

  fn prepare(&mut self, resource: &mut ResourceManager<T>) {
    todo!()
  }
}
