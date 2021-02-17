use rendiation_renderable_mesh::{geometry::*, vertex::Vertex};
use rendiation_scenegraph::default_impl::SceneNodeRenderMatrixData;
use rendiation_webgpu::*;

use rendiation_shader_library::builtin::*;
use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;
use transform::CameraTransform;

use rendiation_ral::{BindGroupHandle, ShaderGeometryInfo, ShaderSampler, ShaderTexture, RAL};

#[derive(Shader)]
pub struct BlockShader {
  parameter: BlockShadingParamGroup,
}

impl ShaderGeometryInfo for BlockShader {
  type Geometry = Vertex;
}

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  #[stage(vert)]
  pub camera: CameraTransform,

  #[stage(vert)]
  pub object: SceneNodeRenderMatrixData,

  #[stage(frag)]
  pub fog: FogData,

  #[stage(frag)]
  pub my_texture_view: ShaderTexture,

  #[stage(frag)]
  pub my_sampler: ShaderSampler,
}

impl ShaderGraphProvider for BlockShader {
  fn build_graph() -> ShaderGraph {
    let (mut builder, input, vertex) = Self::create_builder();
    let p = input.parameter;

    builder.geometry_by::<IndexedGeometry>();

    let (clip_position, mv_position) = mv_p(
      p.camera.projection_matrix,
      p.object.model_view_matrix,
      vertex.position,
      &builder,
    );

    let frag_normal = builder.vary(vertex.normal);
    let frag_uv = builder.vary(vertex.uv);
    let frag_mv_position = builder.vary(mv_position.xyz());

    let block_color = p.my_texture_view.sample(p.my_sampler, frag_uv);

    let block_color = block_color.xyz() * spherical_harmonics(frag_normal);
    let final_color = FogData::apply_fog(p.fog, block_color, length(frag_mv_position));
    builder.set_frag_output(vec4_31(final_color, 1.0));
    builder.create()
  }
}
