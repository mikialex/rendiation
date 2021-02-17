use render_target::{RenderTarget, TargetInfoProvider};
use rendiation_renderable_mesh::{geometry::*, vertex::Vertex};

use rendiation_ral::{ShaderGeometryInfo, ShaderSampler, ShaderTexture, RAL};
use rendiation_shader_library::builtin::*;
use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;

use rendiation_webgpu::*;

#[derive(Shader)]
pub struct CopyShader {
  parameter: CopyParam,
}

impl ShaderGeometryInfo for CopyShader {
  type Geometry = Vertex;
}

impl ShaderGraphProvider for CopyShader {
  fn build_graph() -> ShaderGraph {
    let (mut builder, input, vertex) = CopyShader::create_builder();
    builder.geometry_by::<IndexedGeometry>();
    let parameter = input.parameter;

    builder.set_vertex_root(vec4_31(vertex.position, 1.0));
    let frag_uv = builder.vary(vertex.uv);

    builder.set_frag_output(parameter.my_texture.sample(parameter.my_sampler, frag_uv));

    builder.create()
  }
}

#[derive(BindGroup)]
pub struct CopyParam {
  #[stage(frag)]
  pub my_texture: ShaderTexture,

  #[stage(frag)]
  pub my_sampler: ShaderSampler,
}
