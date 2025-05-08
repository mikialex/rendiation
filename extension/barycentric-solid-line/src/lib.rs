use core::hash::{Hash, Hasher};

use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

/// solid lined mesh is a way to draw on mesh edge line.
///
/// ## references
///
/// https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
/// https://tchayen.github.io/posts/wireframes-with-barycentric-coordinates

#[derive(Clone, Copy)]
struct FullReaderReadWithBarycentric<'a> {
  inner: FullReaderRead<'a>,
  barycentric: Vec3<f32>,
}

impl Eq for FullReaderReadWithBarycentric<'_> {}
impl PartialEq for FullReaderReadWithBarycentric<'_> {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner && self.barycentric == other.barycentric
  }
}

impl Hash for FullReaderReadWithBarycentric<'_> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner.hash(state);
    self.barycentric.map(|f| f.to_bits()).hash(state);
  }
}

pub const BARYCENTRIC_COORD_SEMANTIC_ID: u32 = 100;

pub fn barycentric_shader_inject(id: u32, vertex: &mut ShaderVertexBuilder) {
  if id == BARYCENTRIC_COORD_SEMANTIC_ID {
    vertex.push_single_vertex_layout::<BarycentricCoord>(VertexStepMode::Vertex);
  }
}

impl AttributeVertex for FullReaderReadWithBarycentric<'_> {
  fn layout(&self) -> Vec<AttributeSemantic> {
    let mut inner = self.inner.layout();

    inner.push(AttributeSemantic::Foreign {
      implementation_id: BARYCENTRIC_COORD_SEMANTIC_ID,
      item_byte_size: 3 * 4,
    });
    inner
  }

  fn write(self, target: &mut [Vec<u8>]) {
    self
      .inner
      .write(target.get_mut(0..target.len() - 1).unwrap());
    target
      .last_mut()
      .unwrap()
      .extend_from_slice(bytemuck::bytes_of(&self.barycentric))
  }
}

pub fn generate_barycentric_buffer_and_expanded_mesh(mesh: AttributesMesh) -> AttributesMeshData {
  mesh
    .read()
    .read_full()
    .as_abstract_mesh_read_view()
    .primitive_iter()
    .filter_map(|p| match p {
      AttributeDynPrimitive::Triangle(t) => Some(t),
      _ => None,
    })
    .map(|tri| {
      Triangle::new(
        FullReaderReadWithBarycentric {
          inner: tri.a,
          barycentric: Vec3::new(1., 0., 0.),
        },
        FullReaderReadWithBarycentric {
          inner: tri.b,
          barycentric: Vec3::new(0., 1., 0.),
        },
        FullReaderReadWithBarycentric {
          inner: tri.c,
          barycentric: Vec3::new(0., 0., 1.),
        },
      )
    })
    .collect()
}

pub struct SolidLinedMeshGPU<'a> {
  inner: AttributesMeshGPU<'a>,
}

both!(BarycentricCoord, Vec3<f32>);

impl GraphicsShaderProvider for SolidLinedMeshGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.inner.build(builder);
    builder.vertex(|builder, _| {
      let b = builder.query::<BarycentricCoord>();
      builder.set_vertex_out::<BarycentricCoord>(b);
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let barycentric = builder.query::<BarycentricCoord>();

      let line_color = val(Vec3::zero());
      let smoothing = val(1.);
      let thickness = val(1.);

      let deltas = barycentric.fwidth();
      let smoothing = deltas * smoothing;
      let thickness = deltas * thickness;
      let ratio = barycentric.smoothstep(thickness, thickness + smoothing);
      let ratio = ratio.x().min(ratio.y()).min(ratio.z());

      if let Some(color) = builder.try_query::<ColorChannel>() {
        builder.register::<ColorChannel>(ratio.mix(line_color, color));
      }
    })
  }
}

impl ShaderHashProvider for SolidLinedMeshGPU<'_> {
  shader_hash_type_id! {SolidLinedMeshGPU<'static>}
}
impl ShaderPassBuilder for SolidLinedMeshGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.setup_pass(ctx);
  }
}
