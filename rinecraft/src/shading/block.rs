use rendiation_mesh_buffer::{geometry::*, vertex::Vertex};
use rendiation_webgpu::*;

use render_target::TargetStates;
use rendiation_shadergraph_derives::BindGroup;

use rendiation_shader_library::builtin::*;
use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;
use transform::MVPTransformation;

pub fn create_block_shading(renderer: &WGPURenderer, target: &TargetStates) -> WGPUPipeline {
  let mut builder = ShaderGraphBuilder::new();
  builder.geometry_by::<IndexedGeometry>();

  let vertex = builder.vertex_by::<Vertex>();
  let p = builder.bindgroup_by::<BlockShadingParamGroup>(renderer);

  let mv_position = to_mv_position(vertex.position, p.mvp.model_view);
  let clip_position = apply_projection(mv_position, p.mvp.projection);
  builder.set_vertex_root(clip_position);

  let frag_normal = builder.set_vary(vertex.normal);
  let frag_uv = builder.set_vary(vertex.uv);
  let frag_mv_position = builder.set_vary(mv_position.xyz());

  let block_color = p.my_texture_view.sample(p.my_sampler, frag_uv);

  let block_color = block_color.xyz() * spherical_harmonics(frag_normal);
  let final_color = apply_fog(p.fog, block_color, length(frag_mv_position));
  builder.set_frag_output(vec4_31(final_color, builder.c(1.0)));

  builder
    .create()
    .create_pipeline(renderer)
    .as_mut()
    .target_states(target)
    .build()
}

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  #[stage(vert)]
  pub mvp: MVPTransformation,

  #[stage(frag)]
  pub fog: FogData,

  #[stage(frag)]
  pub my_texture_view: ShaderGraphTexture,

  #[stage(frag)]
  pub my_sampler: ShaderGraphSampler,
}
