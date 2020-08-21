
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
  vec4 copy(
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

    builder.set_vertex_root(position(geometry.position));
    let frag_uv = builder.set_vary(geometry.uv);

    builder.set_frag_output(copy(frag_uv, parameter.sampler, parameter.texture));

    let graph = builder.create();

    let pipeline = PipelineBuilder::new(
      &renderer,
      load_glsl(graph.gen_code_frag(), rendiation_webgpu::ShaderStage::VERTEX),
      load_glsl(graph.gen_code_vertex(), rendiation_webgpu::ShaderStage::FRAGMENT),
    )
    .as_mut()
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
  pub texture: ShaderGraphTexture,

  #[stage(frag)]
  pub sampler: ShaderGraphSampler,
}
