use rendiation_algebra::*;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;
use std::rc::Rc;

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct FatLineMaterial {
  pub width: f32,
}

impl Default for FatLineMaterial {
  fn default() -> Self {
    Self { width: 10. }
  }
}

pub struct FatlineMaterialUniform {
  pub width: f32,
}

pub struct FatlineMaterialGPU {
  _uniform: UniformBuffer<f32>,
}

impl BindGroupLayoutProvider for FatLineMaterial {
  fn bind_preference() -> usize {
    1
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::all(),
        ty: UniformBuffer::<f32>::bind_layout(),
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var<uniform> fatline_material: FatlineMaterial;
    "
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<FatlineMaterialUniform>();
  }
}

impl ShaderGraphProvider for FatlineMaterialGPU {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let pass_info = builder.query_uniform::<RenderPassGPUInfoData>()?.expand();
    let camera = builder.query_uniform::<CameraGPUTransform>()?.expand();
    let model = builder.query_uniform::<TransformGPUData>()?.expand();
    let material = builder.register_uniform::<FatLineMaterial>().expand();

    let resolution = pass_info.buffer_size;
    let aspect = resolution.x() / resolution.y();

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

    // builder.vertex_position.set(clip);
    // builder.set_vertex_out::<FragmentUv>(uv);
    // builder.set_vertex_out::<FragmentColorAndAlpha>(fatline_color);

    Ok(())
  }

  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let vUv = builder.get_fragment_in::<FragmentUv>()?;
    let color = builder.get_fragment_in::<FragmentColorAndAlpha>()?;

    // wgsl!(
    //   // discard corner
    //   let vUv = in.uv;
    //   if ( abs( vUv.y ) > 1.0 ) {
    //     let a = vUv.x;
    //     var b: f32;
    //     if ( vUv.y > 0.0 ) {
    //       b = vUv.y - 1.0;
    //     } else {
    //       b = vUv.y + 1.0;
    //     }
    //     let len2 = a * a + b * b;
    //     if ( len2 > 1.0 ) {
    //       discard;
    //     }
    //   }
    // );

    // // discard corner
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

    builder.set_fragment_out(0, color);
    Ok(())
  }
}

impl WebGPUMaterial for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create_gpu(&self, ctx: &mut SceneMaterialRenderPrepareCtx) -> Self::GPU {
    let device = &gpu.device;
    let _uniform = UniformBuffer::create(device, self.width);

    let bindgroup_layout = Self::layout(device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, ctx.resources, bgw.clone())
      .push(_uniform.as_bindable())
      .build(&bindgroup_layout);

    FatlineMaterialGPU {
      _uniform,
      bindgroup,
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    false
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
