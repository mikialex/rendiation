use crate::{
  GeometryHandle, GeometryProvider, IndexBufferHandle, PrimitiveTopology, ResourceManager,
  ResourceWrap, VertexBufferHandle, RAL,
};
use std::{any::Any, marker::PhantomData, ops::Range};

pub trait GeometryResource<T: RAL>: Any {
  fn apply(&self, render_pass: &mut T::RenderPass, resources: &ResourceManager<T>);
  fn draw(&self, render_pass: &mut T::RenderPass);
  fn get_topology(&self) -> PrimitiveTopology;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait GeometryResourceCreator<T: RAL>: Any {
  type Instance: GeometryResource<T>;
  fn create(
    &self,
    resources: &mut ResourceManager<T>,
    renderer: &mut T::Renderer,
  ) -> Self::Instance;
}

pub trait GeometryResourceInstanceCreator<T: RAL, G: GeometryProvider>:
  GeometryResourceCreator<T, Instance = GeometryResourceInstance<T, G>>
{
  fn create_resource_instance_handle(
    &self,
    renderer: &mut T::Renderer,
    resource: &mut ResourceManager<T>,
  ) -> GeometryHandle<T, G> {
    let instance = self.create(resource, renderer);
    resource.add_geometry(instance)
  }
}

impl<T: RAL, G: GeometryProvider> GeometryResource<T> for GeometryResourceInstance<T, G> {
  fn apply(&self, render_pass: &mut T::RenderPass, resources: &ResourceManager<T>) {
    self.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b).resource();
      T::apply_index_buffer(render_pass, index);
    });
    self.vertex_buffers.iter().enumerate().for_each(|(i, &v)| {
      let vertex = resources.get_vertex_buffer(v).resource();
      T::apply_vertex_buffer(render_pass, i as i32, vertex);
    });
  }

  fn draw(&self, render_pass: &mut T::RenderPass) {
    if self.index_buffer.is_some() {
      T::draw_indexed(render_pass, self.topology, self.draw_range.clone())
    } else {
      T::draw_none_indexed(render_pass, self.topology, self.draw_range.clone())
    }
  }

  fn get_topology(&self) -> PrimitiveTopology {
    self.topology
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
}

pub struct GeometryResourceInstance<T: RAL, G: GeometryProvider> {
  pub draw_range: Range<u32>,
  marker: PhantomData<G>,
  pub index_buffer: Option<IndexBufferHandle<T>>,
  pub vertex_buffers: Vec<VertexBufferHandle<T>>,
  pub topology: PrimitiveTopology,
}

impl<T: RAL, G: GeometryProvider> GeometryResourceInstance<T, G> {
  pub fn new() -> Self {
    Self {
      draw_range: 0..0,
      marker: PhantomData,
      index_buffer: None,
      vertex_buffers: Vec::new(),
      topology: PrimitiveTopology::TriangleList,
    }
  }
}

impl<T: RAL> ResourceManager<T> {
  pub fn add_geometry<G: GeometryProvider>(
    &mut self,
    g: GeometryResourceInstance<T, G>,
  ) -> GeometryHandle<T, G> {
    unsafe { self.geometries.insert(Box::new(g)).cast_type() }
  }

  pub fn get_geometry_mut<G: GeometryProvider>(
    &mut self,
    index: GeometryHandle<T, G>,
  ) -> &mut GeometryResourceInstance<T, G> {
    self
      .geometries
      .get_mut(unsafe { index.cast_type() })
      .unwrap()
      .as_any_mut()
      .downcast_mut::<GeometryResourceInstance<T, G>>()
      .unwrap()
  }

  pub fn get_geometry_boxed<G: GeometryProvider>(
    &self,
    index: GeometryHandle<T, G>,
  ) -> &dyn GeometryResource<T> {
    self
      .geometries
      .get(unsafe { index.cast_type() })
      .unwrap()
      .as_ref()
  }

  pub fn get_geometry<G: GeometryProvider>(
    &self,
    index: GeometryHandle<T, G>,
  ) -> &GeometryResourceInstance<T, G> {
    self
      .geometries
      .get(unsafe { index.cast_type() })
      .unwrap()
      .as_any()
      .downcast_ref::<GeometryResourceInstance<T, G>>()
      .unwrap()
  }

  pub fn delete_geometry<G: GeometryProvider>(&mut self, index: GeometryHandle<T, G>) {
    self.geometries.remove(unsafe { index.cast_type() });
  }

  pub fn delete_geometry_with_buffers<G: GeometryProvider>(&mut self, index: GeometryHandle<T, G>) {
    let geometry = self
      .geometries
      .get(unsafe { index.cast_type() })
      .unwrap()
      .as_any()
      .downcast_ref::<GeometryResourceInstance<T, G>>()
      .unwrap();
    if let Some(b) = geometry.index_buffer {
      self.index_buffers.remove(b);
    }
    for b in &geometry.vertex_buffers {
      self.vertex_buffers.remove(*b);
    }
    self.geometries.remove(unsafe { index.cast_type() });
  }

  pub fn add_index_buffer(&mut self, g: T::IndexBuffer) -> &mut ResourceWrap<T::IndexBuffer> {
    ResourceWrap::new_wrap(&mut self.index_buffers, g)
  }

  pub fn get_index_buffer_mut(
    &mut self,
    index: IndexBufferHandle<T>,
  ) -> &mut ResourceWrap<T::IndexBuffer> {
    self.index_buffers.get_mut(index).unwrap()
  }

  pub fn get_index_buffer(&self, index: IndexBufferHandle<T>) -> &ResourceWrap<T::IndexBuffer> {
    self.index_buffers.get(index).unwrap()
  }

  pub fn delete_index_buffer(&mut self, index: IndexBufferHandle<T>) {
    self.index_buffers.remove(index);
  }

  pub fn add_vertex_buffer(&mut self, g: T::VertexBuffer) -> &mut ResourceWrap<T::VertexBuffer> {
    ResourceWrap::new_wrap(&mut self.vertex_buffers, g)
  }

  pub fn get_vertex_buffer_mut(
    &mut self,
    index: VertexBufferHandle<T>,
  ) -> &mut ResourceWrap<T::VertexBuffer> {
    self.vertex_buffers.get_mut(index).unwrap()
  }

  pub fn get_vertex_buffer(&self, index: VertexBufferHandle<T>) -> &ResourceWrap<T::VertexBuffer> {
    self.vertex_buffers.get(index).unwrap()
  }

  pub fn delete_vertex_buffer(&mut self, index: VertexBufferHandle<T>) {
    self.vertex_buffers.remove(index);
  }
}
