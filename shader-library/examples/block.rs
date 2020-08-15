use rendiation_shader_library::*;
use transform::*;

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  // #[bind_stage = "vertex"]
  pub uniforms: MVPTransformation,

  // #[bind_stage = "fragment"]
  // pub texture_view: TextureView, // need cal stuff?

  // #[bind_stage = "fragment"]
  pub sampler: ShaderGraphSampler,
}

#[repr(C)]
#[derive(Clone, Copy, Geometry)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}

fn main() {
  let mut builder = ShaderGraphBuilder::new();
  let geometry = builder.geometry_by::<Vertex>();
  let block_parameter = builder.bindgroup_by::<BlockShadingParamGroup>();
  let uniforms = block_parameter.uniforms;

  let vertex_position = mvp_projection(geometry.position, uniforms.projection, uniforms.model_view);
  builder.set_vertex_root(vertex_position);

  let frag_normal = builder.set_vary(geometry.normal);

  // builder.set_frag_output(vec4_31(frag_normal, con(1.0)));

  let graph = builder.create();

  println!("{}", graph.gen_code_vertex());
  println!("{}", graph.gen_code_frag());
}
