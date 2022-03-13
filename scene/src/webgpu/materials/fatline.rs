use rendiation_webgpu::*;

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderStruct)]
pub struct FatLineMaterial {
  pub width: f32,
}

impl Default for FatLineMaterial {
  fn default() -> Self {
    Self { width: 10. }
  }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderStruct)]
pub struct FatlineMaterialUniform {
  pub width: f32,
}

pub struct FatlineMaterialGPU {
  uniform: UniformBufferView<FatlineMaterialUniform>,
}

impl ShaderHashProvider for FatlineMaterialGPU {}

impl ShaderPassBuilder for FatlineMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.setup_uniform(&self.uniform, SB::Material);
  }
}

impl ShaderGraphProvider for FatlineMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, binding| {
      // let pass_info = builder.query_uniform::<RenderPassGPUInfoData>()?.expand();
      // let camera = builder.query_uniform::<CameraGPUTransform>()?.expand();
      // let model = builder.query_uniform::<TransformGPUData>()?.expand();
      let material = binding.uniform_by(&self.uniform, SB::Material).expand();

      // let resolution = builder.query::<ViewSize>();
      // let aspect = resolution.x() / resolution.y();

      // builder.vertex_position.set(clip);
      // builder.set_vertex_out::<FragmentUv>(uv);
      // builder.set_vertex_out::<FragmentColorAndAlpha>(fatline_color);

      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let uv = builder.query::<FragmentUv>()?;
      let color = builder.query::<FragmentColorAndAlpha>()?.get();

      // todo corner discard

      builder.set_fragment_out(0, color)
    })
  }
}

// wgsl_function!(
//   fn fatline_vertex(
// start: vec3<f32>,
// end: vec3<f32>,
//  current_point: vec3<f32>
//  view_size: vec2<f32>,
//  width: f32,
// ) -> vec3<f32> {
// // camera space
// let start = camera.view * model.world_matrix * (fatline_start, 1.0).into();
// let end = camera.view * model.world_matrix * (fatline_end, 1.0).into();

// // // special case for perspective projection, and segments that terminate either in, or behind, the camera plane
// // // clearly the gpu firmware has a way of addressing this issue when projecting into ndc space
// // // but we need to perform ndc-space calculations in the shader, so we must address this issue directly
// // // perhaps there is a more elegant solution -- WestLangley
// // bool perspective = ( camera.projection[ 2 ][ 3 ] == - 1.0 ); // 4th entry in the 3rd column
// // if ( perspective ) {{
// //     if ( start.z < 0.0 && end.z >= 0.0 ) {{
// //         trimSegment( start, end );
// //     }} else if ( end.z < 0.0 && start.z >= 0.0 ) {{
// //         trimSegment( end, start );
// //     }}
// // }}

// // clip space
// let clipStart = camera.projection * start;
// let clipEnd = camera.projection * end;

// // ndc space
// let ndcStart = clipStart.xy() / clipStart.w();
// let ndcEnd = clipEnd.xy() / clipEnd.w();

// // direction
// let dir = ndcEnd - ndcStart;

// // account for clip-space aspect ratio
// dir.x = dir.x() * aspect;
// dir = normalize(dir);

// // perpendicular to dir
// let offset = Vec2::new(dir.y, -dir.x);

// // undo aspect ratio adjustment
// dir.x = dir.x / aspect;
// offset.x = offset.x / aspect;

// // sign flip
// if (position.x < 0.0) {
//   {
//     offset = -1.0 * offset;
//   }
// };

// // end caps
// if (position.y < 0.0) {
//   {
//     offset = offset - dir;
//   }
// } else if (position.y > 1.0) {
//   {
//     offset = offset + dir;
//   }
// }

// // adjust for fatLineWidth
// offset = offset * material.width;
// // adjust for clip-space to screen-space conversion // maybe resolution should be based on viewport ...
// offset = offset / resolution.y();

// // select end
// let clip: vec4<f32>;
// if (position.y < 0.5) {
//   {
//     clip = clipStart;
//   }
// } else {
//   {
//     clip = clipEnd;
//   }
// }

// // back to clip space
// offset = offset * clip.w;
// clip = (clip.xy + offset, clip.zw).into();
//   }
// );

// wgsl_function!(
//   fn fatline_round_corner(uv: vec2<f32>)  {
// if (abs(vUv.y) > 1.0) {
//   let a = vUv.x;
//   let b: f32;
//   if (vUv.y > 0.0) {
//     b = vUv.y - 1.0;
//   } else {
//     b = vUv.y + 1.0;
//   }
//   let len2 = a * a + b * b;
//   if (len2 > 1.0) {
//     builder.discard();
//   }
// }
//   }
// );

impl WebGPUMaterial for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = FatlineMaterialUniform { width: self.width };
    let uniform = UniformBufferResource::create_with_source(uniform, &gpu.device);
    let uniform = uniform.create_view(Default::default());

    FatlineMaterialGPU { uniform }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    false
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
