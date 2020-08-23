// use rendiation_shader_library::fog::*;
// use rendiation_shader_library::sph::*;
// use rendiation_shader_library::*;
// use transform::*;

// #[derive(BindGroup)]
// pub struct BlockShadingParamGroup {
//   #[stage(vert)]
//   pub mvp: MVPTransformation,

//   #[stage(frag)]
//   pub fog: FogData,

//   #[stage(frag)]
//   pub my_texture_view: ShaderGraphTexture,

//   #[stage(frag)]
//   pub my_sampler: ShaderGraphSampler,
// }

// // impl BlockShadingParamGroup{
// //   pub fn create_bindgroup(
// //     mvp: UniformHandle<MVPTransformation>,
// //     fog: UniformHandle<FogData>,

// //   ) -> WGPUBindgroup{
// //     ..
// //   }
// // }

// #[repr(C)]
// #[derive(Clone, Copy, Geometry)]
// pub struct Vertex {
//   pub position: Vec3<f32>,
//   pub normal: Vec3<f32>,
//   pub uv: Vec2<f32>,
// }

// glsl_function!(
//   "
//   vec4 block_frag_color(
//       vec3 diffuse,
//       vec2 uv,
//       vec4 mv_position,
//       sampler sa,
//       texture2D tex
//     ){
//     vec3 color = diffuse * spherical_harmonics(v_normal);
//     color *= texture(sampler2D(tex, sa), uv).rgb;
//     float distance = length(mv_position);
//     return vec4(linear_fog(color, distance), 1.0);
//   }
//   "
// );

// struct BlockShader;

// impl BlockShader {
//   pub fn build_shader() {
//     let mut builder = ShaderGraphBuilder::new();
//     let geometry = builder.geometry_by::<Vertex>();
//     let block_parameter = builder.bindgroup_by::<BlockShadingParamGroup>();
//     let uniforms = block_parameter.mvp;

//     let mv_position = to_mv_position(geometry.position, uniforms.model_view);
//     let clip_position = projection(mv_position, uniforms.projection);
//     builder.set_vertex_root(clip_position);

//     let frag_normal = builder.set_vary(geometry.normal);
//     let frag_uv = builder.set_vary(geometry.uv);
//     let frag_mv_position = builder.set_vary(mv_position);

//     builder.set_frag_output(block_frag_color(
//       frag_normal,
//       frag_uv,
//       frag_mv_position,
//       block_parameter.my_sampler,
//       block_parameter.my_texture_view,
//     ));

//     let graph = builder.create();

//     println!("{}", graph.gen_code_vertex());
//     println!("{}", graph.gen_code_frag());
//   }
// }

// fn main() {
//   BlockShader::build_shader();
// }
