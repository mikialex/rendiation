use crate::*;

pub struct AttributesMeshGPU {
  attributes: Vec<(AttributeSemantic, GPUBufferResourceView)>,
  indices: Option<(GPUBufferResourceView, webgpu::IndexFormat)>,
  mode: webgpu::PrimitiveTopology,
  draw: DrawCommand,
}

impl Stream for AttributesMeshGPU {
  type Item = RenderComponentDeltaFlag;
  fn poll_next(self: Pin<&mut Self>, _: &mut Context) -> Poll<Option<Self::Item>> {
    Poll::Pending
  }
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
        AttributeSemantic::Joints(_) => ctx.set_vertex_buffer_owned_next(b),
        AttributeSemantic::Weights(_) => ctx.set_vertex_buffer_owned_next(b),
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
          AttributeSemantic::TexCoords(channel) => match channel {
            // support 3 channel should be enough
            0 => builder.push_single_vertex_layout::<GeometryUVChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<GeometryUVChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<GeometryUVChannel<2>>(mode),
            _ => return Err(ShaderGraphBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Joints(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<JointIndexChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<JointIndexChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<JointIndexChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<JointIndexChannel<3>>(mode),
            _ => return Err(ShaderGraphBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Weights(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<WeightChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<WeightChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<WeightChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<WeightChannel<3>>(mode),
            _ => return Err(ShaderGraphBuildError::SemanticNotSupported),
          },
        }
      }
      builder.primitive_state.topology = self.mode;
      Ok(())
    })
  }
}

struct GPUAttributesBuffer {
  inner: GPUBufferResource,
}

impl SceneItemReactiveSimpleMapping<GPUAttributesBuffer> for GeometryBuffer {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx<'a> = ResourceGPUCtx;

  fn build(&self, gpu: &Self::Ctx<'_>) -> (GPUAttributesBuffer, Self::ChangeStream) {
    let source = self.read();
    let gpu_buffer = create_gpu_buffer(
      self.read().buffer.as_slice(),
      webgpu::BufferUsages::INDEX | webgpu::BufferUsages::VERTEX,
      &gpu.device,
    );

    let gpu_buffer = GPUAttributesBuffer { inner: gpu_buffer };

    let change = source.unbound_listen_by(any_change);
    (gpu_buffer, change)
  }
}

fn get_update_buffer<'a>(
  storage: &'a mut AnyMap,
  source: &GeometryBuffer,
  gpu: &ResourceGPUCtx,
) -> &'a GPUBufferResource {
  let cache: &mut ReactiveMap<GeometryBuffer, GPUAttributesBuffer> =
    storage.entry().or_insert_with(Default::default);
  &cache.get_with_update(source, gpu).inner
}

impl ReactiveRenderComponentSource for AttributesMeshGPUReactive {
  fn as_reactive_component(&self) -> &dyn ReactiveRenderComponent {
    self.as_ref() as &dyn ReactiveRenderComponent
  }
}

impl MeshDrawcallEmitter for AttributesMeshGPUReactive {
  fn draw_command(&self, _group: MeshDrawGroup) -> DrawCommand {
    let inner: &AttributesMeshGPU = self.as_ref();
    inner.draw.clone()
  }
}
/// the current represent do not have meaningful mesh draw group concept
fn draw_command(mesh: &AttributesMesh) -> webgpu::DrawCommand {
  if let Some((_, indices)) = &mesh.indices {
    webgpu::DrawCommand::Indexed {
      base_vertex: 0,
      indices: 0..indices.count as u32,
      instances: 0..1,
    }
  } else {
    let attribute = &mesh.attributes.last().unwrap().1;
    webgpu::DrawCommand::Array {
      vertices: 0..attribute.count as u32,
      instances: 0..1,
    }
  }
}

type AttributesMeshGPUReactive =
  impl AsRef<RenderComponentCell<AttributesMeshGPU>> + Stream<Item = RenderComponentDeltaFlag>;

impl WebGPUMesh for AttributesMesh {
  type ReactiveGPU = AttributesMeshGPUReactive;

  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU {
    let weak = source.downgrade();
    let ctx = ctx.clone();

    let create = move || {
      if let Some(m) = weak.upgrade() {
        let mut custom_storage = ctx.custom_storage.write().unwrap();
        let mesh = m.read();
        let attributes = mesh
          .attributes
          .iter()
          .map(|(s, vertices)| {
            let buffer = get_update_buffer(&mut custom_storage, &vertices.view.buffer, &ctx.gpu);
            let buffer_view = buffer.create_view(map_view(vertices.compute_gpu_buffer_range()));
            (s.clone(), buffer_view)
          })
          .collect();

        let indices = mesh.indices.as_ref().map(|(format, i)| {
          let buffer = get_update_buffer(&mut custom_storage, &i.view.buffer, &ctx.gpu);
          let buffer_view = buffer.create_view(map_view(i.compute_gpu_buffer_range()));
          (buffer_view, map_index(*format))
        });

        let r = AttributesMeshGPU {
          attributes,
          indices,
          mode: map_topology(mesh.mode),
          draw: draw_command(&mesh),
        };

        Some(r)
      } else {
        None
      }
    };

    let gpu = create().unwrap();
    let state = RenderComponentCell::new(gpu);

    source
      .single_listen_by::<()>(any_change_no_init)
      .fold_signal(state, move |_, state| {
        if let Some(gpu) = create() {
          state.inner = gpu;
          RenderComponentDeltaFlag::all().into()
        } else {
          None
        }
      })
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
