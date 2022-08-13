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
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FatlineMaterialUniform {
  pub width: f32,
}

pub struct FatlineMaterialGPU {
  uniform: UniformBufferView<FatlineMaterialUniform>,
}

impl ShaderHashProvider for FatlineMaterialGPU {}

impl ShaderPassBuilder for FatlineMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
  }
}

impl ShaderGraphProvider for FatlineMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, binding| {
      let uv = builder.query::<GeometryUV>()?.get();
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>()?.get();
      let material = binding.uniform_by(&self.uniform, SB::Material).expand();

      let vertex_position = fatline_vertex(
        builder.query::<CameraProjectionMatrix>()?.get(),
        builder.query::<CameraViewMatrix>()?.get(),
        builder.query::<WorldMatrix>()?.get(),
        builder.query::<FatLineStart>()?.get(),
        builder.query::<FatLineEnd>()?.get(),
        builder.query::<GeometryPosition>()?.get(),
        builder.query::<RenderBufferSize>()?.get(),
        material.width,
      );

      builder.register::<ClipPosition>(vertex_position);
      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<FragmentColorAndAlpha>(color_with_alpha);
      Ok(())
    })?;

    builder.fragment(|builder, _| {
      let uv = builder.query::<FragmentUv>()?.get();
      let color = builder.query::<FragmentColorAndAlpha>()?.get();

      if_by(discard_fatline_round_corner(uv), || {
        // builder.discard(); todo!
      });

      builder.set_fragment_out(0, color)
    })
  }
}

wgsl_fn!(
  fn fatline_vertex(
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
    world_matrix: mat4x4<f32>,
    fatline_start: vec3<f32>,
    fatline_end: vec3<f32>,
    position: vec3<f32>,
    view_size: vec2<f32>,
    width: f32,
  ) -> vec4<f32> {
      // camera space
      let start = view * world_matrix * vec4<f32>(fatline_start, 1.0);
      let end = view * world_matrix * vec4<f32>(fatline_end, 1.0);

      // // special case for perspective projection, and segments that terminate either in, or behind, the camera plane
      // // clearly the gpu firmware has a way of addressing this issue when projecting into ndc space
      // // but we need to perform ndc-space calculations in the shader, so we must address this issue directly
      // // perhaps there is a more elegant solution -- WestLangley
      // bool perspective = ( projection[ 2 ][ 3 ] == - 1.0 ); // 4th entry in the 3rd column
      // if ( perspective ) {{
      //     if ( start.z < 0.0 && end.z >= 0.0 ) {{
      //         trimSegment( start, end );
      //     }} else if ( end.z < 0.0 && start.z >= 0.0 ) {{
      //         trimSegment( end, start );
      //     }}
      // }}

      let aspect = view_size.x / view_size.y;

      // clip space
      let clipStart = projection * start;
      let clipEnd = projection * end;

      // ndc space
      let ndcStart = clipStart.xy / clipStart.w;
      let ndcEnd = clipEnd.xy / clipEnd.w;

      // direction
      var dir = ndcEnd - ndcStart;

      // account for clip-space aspect ratio
      dir.x = dir.x * aspect;
      dir = normalize(dir);

      // perpendicular to dir
      var offset = vec2<f32>(dir.y, -dir.x);

      // undo aspect ratio adjustment
      dir.x = dir.x / aspect;
      offset.x = offset.x / aspect;

      // sign flip
      if (position.x < 0.0) {
          offset = -1.0 * offset;
      };

      // end caps
      if (position.y < 0.0) {
          offset = offset - dir;
      } else if (position.y > 1.0) {
          offset = offset + dir;
      }

      // adjust for fatLineWidth
      offset = offset * width;
      // adjust for clip-space to screen-space conversion // maybe resolution should be based on viewport ...
      offset = offset / view_size.y;

      // select end
      var clip: vec4<f32>;
      if (position.y < 0.5) {
          clip = clipStart;
      } else {
          clip = clipEnd;
      }

      // back to clip space
      offset = offset * clip.w;
      return vec4<f32>(clip.xy + offset, clip.zw);
  }
);

wgsl_fn!(
  fn discard_fatline_round_corner(uv: vec2<f32>) -> bool {
    if (abs(vUv.y) > 1.0) {
      let a = vUv.x;
      let b: f32;
      if (vUv.y > 0.0) {
        b = vUv.y - 1.0;
      } else {
        b = vUv.y + 1.0;
      }
      let len2 = a * a + b * b;
      if (len2 > 1.0) {
        return true;
      }
    }
    return false;
  }
);

impl WebGPUMaterial for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create_gpu(&self, _: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU {
    let uniform = FatlineMaterialUniform {
      width: self.width,
      ..Zeroable::zeroed()
    };
    let uniform = UniformBufferResource::create_with_source(uniform, &gpu.device);
    let uniform = uniform.create_default_view();

    FatlineMaterialGPU { uniform }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    false
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
