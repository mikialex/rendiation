use render_target::{RenderTarget, TargetStatesProvider};
use rendiation_mesh_buffer::{geometry::*, vertex::Vertex};

use rendiation_shader_library::builtin::*;
use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;

use rendiation_webgpu::*;

pub struct CopierShading {
  pub pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let mut builder = ShaderGraphBuilder::new();
    builder.geometry_by::<IndexedGeometry>();
    let geometry = builder.vertex_by::<Vertex>();

    let parameter = builder.bindgroup_by::<CopyParam>(renderer);

    builder.set_vertex_root(vec4_31(geometry.position, builder.c(1.0)));
    let frag_uv = builder.set_vary(geometry.uv);

    builder.set_frag_output(parameter.my_texture.sample(parameter.my_sampler, frag_uv));

    let graph = builder.create();

    Self {
      pipeline: graph
        .create_pipeline(renderer)
        .target_states(target.create_target_states().as_ref())
        .build(),
    }
  }
}

#[derive(BindGroup)]
pub struct CopyParam {
  #[stage(frag)]
  pub my_texture: ShaderGraphTexture,

  #[stage(frag)]
  pub my_sampler: ShaderGraphSampler,
}
