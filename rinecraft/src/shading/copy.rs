use render_target::{RenderTarget, TargetStatesProvider};
use rendiation_mesh_buffer::{geometry::*, vertex::Vertex};

use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;

use rendiation_webgpu::*;

pub struct CopierShading {
  pub pipeline: WGPUPipeline,
}

glsl_function!(
  "
  vec4 just_copy(
      vec2 uv,
      sampler sa,
      texture2D tex
    ){
    return texture(sampler2D(tex, sa), uv); 
  }
  "
);

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let mut builder = ShaderGraphBuilder::new();
    let geometry = builder.geometry_by::<Vertex>();

    let parameter = builder.bindgroup_by::<CopyParam>();

    builder.set_vertex_root(make_position(geometry.position));
    let frag_uv = builder.set_vary(geometry.uv);

    builder.set_frag_output(just_copy(
      frag_uv,
      parameter.my_sampler,
      parameter.my_texture,
    ));

    let graph = builder.create();

    let pipeline = graph
      .create_pipeline(renderer)
      .binding_group::<CopyParam>()
      .geometry::<IndexedGeometry>()
      .target_states(target.create_target_states().as_ref())
      .build();

    Self { pipeline }
  }
}

#[derive(BindGroup)]
pub struct CopyParam {
  #[stage(frag)]
  pub my_texture: ShaderGraphTexture,

  #[stage(frag)]
  pub my_sampler: ShaderGraphSampler,
}
