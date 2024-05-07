use crate::*;

pub type WideLineMesh = NoneIndexedMesh<LineList, Vec<WideLineVertex>>;

pub struct WideLineMeshGPU {
  vertex: GPUBufferResourceView,
  /// All wide_line gpu instance shall share one instance buffer
  // instance: Rc<MeshGPU>,
  range_full: MeshGroup,
}

impl WideLineMeshGPU {
  pub fn draw_command(&self) -> DrawCommand {
    let range = self.range_full;

    LINE_SEG_INSTANCE.with(|instance| DrawCommand::Indexed {
      base_vertex: 0,
      indices: 0..instance.draw_count() as u32,
      instances: range.into(),
    })
  }

  pub fn new(mesh: WideLineMesh, device: &GPUDevice) -> WideLineMeshGPU {
    let vertex = bytemuck::cast_slice(mesh.data.as_slice());
    let vertex = create_gpu_buffer(vertex, BufferUsages::VERTEX, device).create_default_view();

    // let instance = ctx
    //   .custom_storage
    //   .write()
    //   .unwrap()
    //   .entry()
    //   .or_insert_with(|| create_wide_line_quad_gpu(&ctx.gpu.device))
    //   .data
    //   .clone();

    let range_full = MeshGroup {
      start: 0,
      count: mesh.draw_count(),
    };

    WideLineMeshGPU {
      vertex,
      // instance,
      range_full,
    }
  }
}

impl GraphicsShaderProvider for WideLineMeshGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<Vertex>(VertexStepMode::Vertex);
      builder.register_vertex::<WideLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
      Ok(())
    })
  }
}

impl ShaderHashProvider for WideLineMeshGPU {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for WideLineMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    // self.instance.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.vertex);
  }
}

use bytemuck::{Pod, Zeroable};
use rendiation_mesh_core::{vertex::Vertex, NoneIndexedMesh};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideLineVertex {
  #[semantic(WideLineStart)]
  pub start: Vec3<f32>,
  #[semantic(WideLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

only_vertex!(WideLineStart, Vec3<f32>);
only_vertex!(WideLineEnd, Vec3<f32>);

pub struct WideLineQuadInstance {
  // data: Rc<MeshGPU>,
}

fn create_wide_line_quad() -> IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> {
  #[rustfmt::skip]
  let positions: Vec<isize> = vec![- 1, 2, 0, 1, 2, 0, - 1, 1, 0, 1, 1, 0, - 1, 0, 0, 1, 0, 0, - 1, - 1, 0, 1, - 1, 0];
  let positions: &[Vec3<isize>] = bytemuck::cast_slice(positions.as_slice());
  let uvs: Vec<isize> = vec![-1, 2, 1, 2, -1, 1, 1, 1, -1, -1, 1, -1, -1, -2, 1, -2];
  let uvs: &[Vec2<isize>] = bytemuck::cast_slice(uvs.as_slice());

  let data: Vec<_> = positions
    .iter()
    .zip(uvs)
    .map(|(position, uv)| Vertex {
      position: position.map(|v| v as f32),
      normal: Vec3::new(0., 0., 1.),
      uv: uv.map(|v| v as f32),
    })
    .collect();

  let index = vec![0, 2, 1, 2, 3, 1, 2, 4, 3, 4, 5, 3, 4, 6, 5, 6, 7, 5];
  IndexedMesh::new(data, index)
}

thread_local! {
  static LINE_SEG_INSTANCE: IndexedMesh<TriangleList, Vec<Vertex>, Vec<u16>> = create_wide_line_quad()
}

// fn create_wide_line_quad_gpu(device: &GPUDevice) -> WideLineQuadInstance {
//   WideLineQuadInstance {
//     data: Rc::new(LINE_SEG_INSTANCE.with(|f| create_gpu(f, device, Default::default()))),
//   }
// }
