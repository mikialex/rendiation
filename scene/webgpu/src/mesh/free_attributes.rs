use crate::*;

pub struct AttributesMeshGPU {
  attributes: Vec<(AttributeSemantic, GPUBufferResourceView)>,
  indices: Option<(GPUBufferResourceView, webgpu::IndexFormat)>,
  mode: webgpu::PrimitiveTopology,
}

impl ShaderPassBuilder for AttributesMeshGPU {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    for (s, b) in &self.attributes {
      match s {
        AttributeSemantic::Positions => ctx.set_vertex_buffer_owned_next(b),
        AttributeSemantic::Normals => ctx.set_vertex_buffer_owned_next(b),
        AttributeSemantic::Tangents => {}
        AttributeSemantic::Colors(_) => ctx.set_vertex_buffer_owned_next(b),
        AttributeSemantic::TexCoords(_) => ctx.set_vertex_buffer_owned_next(b),
        AttributeSemantic::Joints(_) => {}
        AttributeSemantic::Weights(_) => {}
      }
    }
    if let Some((buffer, index_format)) = &self.indices {
      ctx.pass.set_index_buffer_owned(buffer, *index_format)
    }
  }
}

impl ShaderHashProvider for AttributesMeshGPU {
  fn hash_pipeline(&self, hasher: &mut webgpu::PipelineHasher) {
    for (s, _) in &self.attributes {
      s.hash(hasher)
    }
    self.mode.hash(hasher);
    if let Some((_, f)) = &self.indices {
      if webgpu::PrimitiveTopology::LineStrip == self.mode
        || webgpu::PrimitiveTopology::TriangleStrip == self.mode
      {
        f.hash(hasher)
      }
    }
  }
}
impl ShaderGraphProvider for AttributesMeshGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let mode = VertexStepMode::Vertex;
    builder.vertex(|builder, _| {
      for (s, _) in &self.attributes {
        match s {
          AttributeSemantic::Positions => {
            builder.push_single_vertex_layout::<GeometryPosition>(mode)
          }
          AttributeSemantic::Normals => builder.push_single_vertex_layout::<GeometryNormal>(mode),
          AttributeSemantic::Tangents => {}
          AttributeSemantic::Colors(_) => builder.push_single_vertex_layout::<GeometryColor>(mode),
          AttributeSemantic::TexCoords(_) => builder.push_single_vertex_layout::<GeometryUV>(mode),
          AttributeSemantic::Joints(_) => {}
          AttributeSemantic::Weights(_) => {}
        }
      }
      builder.primitive_state.topology = self.mode;
      Ok(())
    })
  }
}

fn get_update_buffer<'a>(
  storage: &'a mut AnyMap,
  source: &GeometryBuffer,
  gpu: &GPU,
) -> &'a GPUBufferResource {
  let cache: &mut IdentityMapper<GPUBufferResource, GeometryBufferInner> =
    storage.entry().or_insert_with(Default::default);

  cache.get_update_or_insert_with(
    &source.read(),
    |buffer| {
      create_gpu_buffer(
        buffer.buffer.as_slice(),
        webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
        &gpu.device,
      )
    },
    |g, buffer| {
      *g = create_gpu_buffer(
        buffer.buffer.as_slice(),
        webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
        &gpu.device,
      )
    },
  )
}

impl WebGPUMesh for AttributesMesh {
  type GPU = AttributesMeshGPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut AnyMap) -> Self::GPU {
    let attributes = self
      .attributes
      .iter()
      .map(|(s, vertices)| {
        let buffer = get_update_buffer(storage, &vertices.view.buffer, gpu);
        let buffer_view = buffer.create_view(map_view(vertices.compute_gpu_buffer_range()));
        (s.clone(), buffer_view)
      })
      .collect();

    let indices = self.indices.as_ref().map(|(format, i)| {
      let buffer = get_update_buffer(storage, &i.view.buffer, gpu);
      let buffer_view = buffer.create_view(map_view(i.compute_gpu_buffer_range()));
      (buffer_view, map_index(*format))
    });

    AttributesMeshGPU {
      attributes,
      indices,
      mode: map_topology(self.mode),
    }
  }

  /// the current represent do not have meaningful mesh draw group concept
  fn draw_impl(&self, _group: MeshDrawGroup) -> webgpu::DrawCommand {
    if let Some((_, indices)) = &self.indices {
      webgpu::DrawCommand::Indexed {
        base_vertex: 0,
        indices: 0..indices.count as u32,
        instances: 0..1,
      }
    } else {
      let attribute = &self.attributes.last().unwrap().1;
      webgpu::DrawCommand::Array {
        vertices: 0..attribute.count as u32,
        instances: 0..1,
      }
    }
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    map_topology(self.mode)
  }
}

fn map_view(view: BufferViewRange) -> GPUBufferViewRange {
  GPUBufferViewRange {
    offset: view.offset,
    size: view.size,
  }
}

fn map_index(index: IndexFormat) -> webgpu::IndexFormat {
  match index {
    IndexFormat::Uint16 => webgpu::IndexFormat::Uint16,
    IndexFormat::Uint32 => webgpu::IndexFormat::Uint32,
  }
}
