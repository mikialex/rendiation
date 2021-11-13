use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;
use std::rc::Rc;

use crate::*;

#[derive(Clone)]
pub struct FatLineMaterial {
  pub width: f32,
  pub states: MaterialStates,
}

pub struct FatlineMaterialGPU {
  _uniform: UniformBuffer<f32>,
  bindgroup: MaterialBindGroup,
}

impl BindGroupLayoutProvider for FatLineMaterial {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: UniformBuffer::<f32>::bind_layout(),
        count: None,
      }],
    })
  }
}

impl FatLineMaterial {
  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct FatlineMaterial {
      width: f32;
    };

    [[group(1), binding(0)]]
    var<uniform> fatline_material: FatlineMaterial;
    "
  }
}

impl PipelineRequester for FatlineMaterialGPU {
  type Container = PipelineUnit;
}

impl MaterialGPUResource for FatlineMaterialGPU {
  type Source = FatLineMaterial;

  fn pipeline_key(
    &self,
    _source: &Self::Source,
    _ctx: &PipelineCreateCtx,
  ) -> <Self::Container as PipelineVariantContainer>::Key {
  }
  fn create_pipeline(
    &self,
    _source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) {
    let vertex_header = format!(
      "
    {}
    {}
    ",
      Vertex::get_shader_header(),
      FatLineVertex::get_shader_header()
    );

    builder.shader_source = format!(
      "
      {object_header}
      {material_header}
      {camera_header}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] uv: vec2<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        {vertex_header}
      ) -> VertexOutput {{
        var out: VertexOutput;
        
        float aspect = resolution.x / resolution.y;
        // camera space
        vec4 start =  camera.view * model.matrix * vec4( fatline_start, 1.0 );
        vec4 end =  camera.view * model.matrix * vec4( fatline_end, 1.0 );

        // // special case for perspective projection, and segments that terminate either in, or behind, the camera plane
        // // clearly the gpu firmware has a way of addressing this issue when projecting into ndc space
        // // but we need to perform ndc-space calculations in the shader, so we must address this issue directly
        // // perhaps there is a more elegant solution -- WestLangley
        // bool perspective = ( camera.projection[ 2 ][ 3 ] == - 1.0 ); // 4th entry in the 3rd column
        // if ( perspective ) {{
        //     if ( start.z < 0.0 && end.z >= 0.0 ) {{
        //         trimSegment( start, end );
        //     }} else if ( end.z < 0.0 && start.z >= 0.0 ) {{
        //         trimSegment( end, start );
        //     }}
        // }}

        // clip space
        vec4 clipStart = camera.projection * start;
        vec4 clipEnd = camera.projection * end;

        // ndc space
        vec2 ndcStart = clipStart.xy / clipStart.w;
        vec2 ndcEnd = clipEnd.xy / clipEnd.w;

        // direction
        vec2 dir = ndcEnd - ndcStart;

        // account for clip-space aspect ratio
        dir.x *= aspect;
        dir = normalize( dir );

        // perpendicular to dir
        vec2 offset = vec2( dir.y, - dir.x );

        // undo aspect ratio adjustment
        dir.x /= aspect;
        offset.x /= aspect;

        // sign flip
        if ( position.x < 0.0 ) offset *= - 1.0;
        // end caps
        if ( position.y < 0.0 )  {{
            offset += - dir;
        }} else if ( position.y > 1.0 )  {{
            offset += dir;
        }}

        // adjust for fatLineWidth
        offset *= fatLineWidth;
        // adjust for clip-space to screen-space conversion // maybe resolution should be based on viewport ...
        offset /= resolution.y;
        // select end
        vec4 clip = ( position.y < 0.5 ) ? clipStart : clipEnd;
        // back to clip space
        offset *= clip.w;
        clip.xy += offset;

        out.position = clip;

        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return vec4(1., 0., 0., 1.);
      }}
      
      ",
      vertex_header = vertex_header,
      material_header = FatLineMaterial::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = TransformGPU::get_shader_header(),
    );

    builder.with_layout(ctx.layouts.retrieve::<FatLineMaterial>(device));

    builder.vertex_buffers = ctx.active_mesh.unwrap().vertex_layout();
  }

  fn setup_pass_bindgroup<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx,
  ) {
    pass.set_bind_group_owned(1, &self.bindgroup.gpu, &[]);
  }
}

impl MaterialCPUResource for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create(
    &mut self,
    gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let device = &gpu.device;
    let _uniform = UniformBuffer::create(device, self.width);

    let bindgroup_layout = Self::layout(device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, bgw.clone())
      .push(_uniform.gpu().as_entire_binding())
      .build(&bindgroup_layout);

    FatlineMaterialGPU {
      _uniform,
      bindgroup,
    }
  }
  fn is_keep_mesh_shape(&self) -> bool {
    false
  }
}
