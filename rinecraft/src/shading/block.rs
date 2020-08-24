use rendiation_mesh_buffer::{geometry::*, vertex::Vertex};
use rendiation_webgpu::*;

use render_target::TargetStates;
use rendiation_shadergraph_derives::BindGroup;

use rendiation_shader_library::fog::*;
use rendiation_shader_library::sph::*;
use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;
use transform::MVPTransformation;

glsl_function!(
  "
  vec3 block_frag_color(
      vec3 normal,
      vec2 uv,
      sampler sa,
      texture2D tex
    ){
    vec3 color = texture(sampler2D(tex, sa), uv).rgb;
    color *= spherical_harmonics(normal); 
    return color;
  }
  "
);

glsl_function!(
  "
  vec4 apply_fog(
      vec3 color,
      vec4 fog_color,
      vec4 mv_position,
      float fog_start,
      float fog_end
    ){
    float distance = length(mv_position);
    return vec4(linear_fog(color, fog_color, distance, fog_start, fog_end), 1.0);
  }
  "
);

pub fn create_block_shading(renderer: &WGPURenderer, target: &TargetStates) -> WGPUPipeline {
  let mut builder = ShaderGraphBuilder::new();
  let geometry = builder.geometry_by::<Vertex>();
  let p = builder.bindgroup_by::<BlockShadingParamGroup>();

  let mv_position = to_mv_position(geometry.position, p.mvp.model_view);
  let clip_position = apply_projection(mv_position, p.mvp.projection);
  builder.set_vertex_root(clip_position);

  let frag_normal = builder.set_vary(geometry.normal);
  let frag_uv = builder.set_vary(geometry.uv);
  let frag_mv_position = builder.set_vary(mv_position);

  let block_color = block_frag_color(frag_normal, frag_uv, p.my_sampler, p.my_texture_view);

  builder.set_frag_output(apply_fog(
    block_color,
    p.fog.fog_color,
    frag_mv_position,
    p.fog.fog_start,
    p.fog.fog_end,
  ));

  let graph = builder.create();
  graph
    .create_pipeline(renderer)
    .as_mut()
    .binding_group::<BlockShadingParamGroup>()
    .geometry::<IndexedGeometry>()
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
