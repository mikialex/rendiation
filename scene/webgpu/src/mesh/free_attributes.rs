use crate::*;

/// Vertex attribute semantic name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AttributeSemantic {
  /// Extra attribute name.
  #[cfg(feature = "extras")]
  Extras(String),

  /// XYZ vertex positions.
  Positions,

  /// XYZ vertex normals.
  Normals,

  /// XYZW vertex tangents where the `w` component is a sign value indicating the
  /// handedness of the tangent basis.
  Tangents,

  /// RGB or RGBA vertex color.
  Colors(u32),

  /// UV texture co-ordinates.
  TexCoords(u32),

  /// Joint indices.
  Joints(u32),

  /// Joint weights.
  Weights(u32),
}

static GLOBAL_BUFFER_ID: __core::sync::atomic::AtomicU64 = __core::sync::atomic::AtomicU64::new(0);

pub struct GeometryBuffer {
  guid: u64,
  buffer: Vec<u8>,
}

impl GeometryBuffer {
  pub fn new(buffer: Vec<u8>) -> Self {
    Self {
      guid: GLOBAL_BUFFER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
      buffer,
    }
  }
}

/// like slice, but owned, ref counted cheap clone
#[derive(Clone)]
pub struct TypedBufferView {
  pub buffer: Rc<GeometryBuffer>,
  pub range: GPUBufferViewRange,
}

#[derive(Clone)]
pub struct AttributeAccessor {
  pub view: TypedBufferView,
  pub start: usize,
  pub count: usize,
  pub stride: usize,
}

impl AttributeAccessor {
  fn compute_gpu_buffer_range(&self) -> GPUBufferViewRange {
    let inner_offset = self.view.range.offset;
    GPUBufferViewRange {
      offset: inner_offset + (self.start * self.stride) as u64,
      size: NonZeroU64::new(inner_offset + (self.count * self.stride) as u64)
        .unwrap() // safe
        .into(),
    }
  }
}

pub struct AttributesMesh {
  pub attributes: Vec<(AttributeSemantic, AttributeAccessor)>,
  pub indices: Option<(webgpu::IndexFormat, AttributeAccessor)>,
  pub mode: webgpu::PrimitiveTopology,
  // bounding: Box3<f32>,
}

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

// todo impl drop, cleanup
#[derive(Default)]
struct AttributesGPUCache {
  gpus: HashMap<u64, GPUBufferResource>,
}

impl AttributesGPUCache {
  pub fn get(&mut self, buffer: &GeometryBuffer, gpu: &webgpu::GPU) -> GPUBufferResource {
    self
      .gpus
      .entry(buffer.guid)
      .or_insert_with(|| {
        create_gpu_buffer(
          buffer.buffer.as_slice(),
          webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
          &gpu.device,
        )
      })
      .clone()
  }
}

impl WebGPUMesh for AttributesMesh {
  type GPU = AttributesMeshGPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut AnyMap) -> Self::GPU {
    let cache: &mut AttributesGPUCache = storage.entry().or_insert_with(Default::default);

    let attributes = self
      .attributes
      .iter()
      .map(|(s, vertices)| {
        let buffer = cache.get(&vertices.view.buffer, gpu);
        let buffer_view = buffer.create_view(vertices.compute_gpu_buffer_range());
        (s.clone(), buffer_view)
      })
      .collect();

    let indices = self.indices.as_ref().map(|(format, i)| {
      let buffer = cache.get(&i.view.buffer, gpu);
      let buffer_view = buffer.create_view(i.compute_gpu_buffer_range());
      (buffer_view, *format)
    });

    AttributesMeshGPU {
      attributes,
      indices,
      mode: self.mode,
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
    self.mode
  }
}
