use rendiation_shader_library::*;

#[derive(UniformBuffer)]
#[repr(align(16))]
pub struct MVPTransformation {
  pub projection: Mat4<f32>,
  pub model_view: Mat4<f32>,
}

glsl_function!("
vec4 mvp_projection(vec3 raw, mat4 projection, mat4 model_view){
    return projection * model_view * vec4(raw, 1.0);
}
");

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
    let block_paramter = builder.bindgroup_by::<BlockShadingParamGroup>();
    let uniforms = block_paramter.uniforms;

    let vertex_position = mvp_projection(geometry.position, uniforms.projection, uniforms.model_view);
    builder.set_vertex_root(vertex_position);

    builder.set_vary(geometry.normal);

    let graph = builder.create();

    println!("{}", graph.gen_code_vertex());
    println!("{}", graph.gen_code_frag());
}