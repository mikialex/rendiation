use core::hash::{Hash, Hasher};

use dyn_downcast::*;
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

impl<'a> Eq for FullReaderReadWithBarycentric<'a> {}
impl<'a> PartialEq for FullReaderReadWithBarycentric<'a> {
  fn eq(&self, other: &Self) -> bool {
    self.inner == other.inner && self.barycentric == other.barycentric
  }
}

impl<'a> Hash for FullReaderReadWithBarycentric<'a> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inner.hash(state);
    self.barycentric.map(|f| f.to_bits()).hash(state);
  }
}

impl<'a> AttributeVertex for FullReaderReadWithBarycentric<'a> {
  fn layout(&self) -> Vec<AttributeSemantic> {
    let mut inner = self.inner.layout();

    get_dyn_trait_downcaster_static!(CustomAttributeKeyGPU)
      .register::<BarycentricCoordAttributeKey>();

    inner.push(AttributeSemantic::Foreign(ForeignAttributeKey::new(
      BarycentricCoordAttributeKey,
    )));
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

#[derive(Clone, Copy)]
struct BarycentricCoordAttributeKey;

type_as_dyn_trait!(BarycentricCoordAttributeKey, AttributeReadSchema);
impl AttributeReadSchema for BarycentricCoordAttributeKey {
  fn item_byte_size(&self) -> usize {
    3 * 4
  }
}

type_as_dyn_trait!(BarycentricCoordAttributeKey, CustomAttributeKeyGPU);
impl CustomAttributeKeyGPU for BarycentricCoordAttributeKey {
  fn inject_shader(&self, builder: &mut ShaderVertexBuilder) {
    builder.push_single_vertex_layout::<BarycentricCoord>(VertexStepMode::Vertex);
  }
}

pub struct SolidLinedMeshGPU<'a> {
  inner: AttributesMeshGPU<'a>,
}

both!(BarycentricCoord, Vec3<f32>);

impl<'a> GraphicsShaderProvider for SolidLinedMeshGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.inner.build(builder)?;
    builder.vertex(|builder, _| {
      builder.set_vertex_out::<BarycentricCoord>(builder.query::<BarycentricCoord>().unwrap());
      Ok(())
    })
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      let barycentric = builder.query::<BarycentricCoord>().unwrap();

      let line_color = val(Vec3::zero());
      let smoothing = val(1.);
      let thickness = val(1.);

      let deltas = barycentric.fwidth();
      let smoothing = deltas * smoothing;
      let thickness = deltas * thickness;
      let ratio = barycentric.smoothstep(thickness, thickness + smoothing);
      let ratio = ratio.x().min(ratio.y()).min(ratio.z());

      if let Ok(color) = builder.query::<ColorChannel>() {
        builder.register::<ColorChannel>(ratio.mix(line_color, color));
      }

      Ok(())
    })
  }
}

impl<'a> ShaderHashProvider for SolidLinedMeshGPU<'a> {
  shader_hash_type_id! {SolidLinedMeshGPU<'static>}
}
impl<'a> ShaderPassBuilder for SolidLinedMeshGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.inner.setup_pass(ctx);
  }
}
