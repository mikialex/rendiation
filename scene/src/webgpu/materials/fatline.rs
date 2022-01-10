use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;
use std::rc::Rc;

use crate::*;

#[derive(Clone)]
pub struct FatLineMaterial {
  pub width: f32,
  pub states: MaterialStates,
}

impl Default for FatLineMaterial {
  fn default() -> Self {
    Self {
      width: 10.,
      states: Default::default(),
    }
  }
}

pub struct FatlineMaterialUniform {
  pub width: f32,
}

impl ShaderUniformBlock for FatlineMaterialUniform {
  fn shader_struct() -> &'static str {
    "
      struct FatlineMaterial {
        width: f32;
      };
      "
  }
}

pub struct FatlineMaterialGPU {
  _uniform: UniformBuffer<f32>,
  bindgroup: MaterialBindGroup,
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

impl MaterialGPUResource for FatlineMaterialGPU {
  type Source = FatLineMaterial;

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

    builder.include_vertex_entry(format!("
      [[stage(vertex)]]
      fn vs_fatline_main(
        {vertex_header}
      ) -> FatlineVertexOutput {{
        var out: FatlineVertexOutput;
        
        let resolution = pass_info.buffer_size;

        let aspect = resolution.x / resolution.y;
        // camera space
        let start =  camera.view * model.matrix * vec4<f32>( fatline_start, 1.0 );
        let end =  camera.view * model.matrix * vec4<f32>( fatline_end, 1.0 );

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
        let clipStart = camera.projection * start;
        let clipEnd = camera.projection * end;

        // ndc space
        let ndcStart = clipStart.xy / clipStart.w;
        let ndcEnd = clipEnd.xy / clipEnd.w;

        // direction
        var dir = ndcEnd - ndcStart;

        // account for clip-space aspect ratio
        dir.x = dir.x * aspect;
        dir = normalize( dir );

        // perpendicular to dir
        var offset = vec2<f32>( dir.y, - dir.x );

        // undo aspect ratio adjustment
        dir.x = dir.x / aspect;
        offset.x = offset.x / aspect;

        // sign flip
        if ( position.x < 0.0 ) {{
          offset = - 1.0 * offset;
        }};
        
        // end caps
        if ( position.y < 0.0 )  {{
            offset = offset - dir;
        }} else if ( position.y > 1.0 )  {{
            offset = offset + dir;
        }}

        // adjust for fatLineWidth
        offset = offset * fatline_material.width;
        // adjust for clip-space to screen-space conversion // maybe resolution should be based on viewport ...
        offset = offset / resolution.y;

        // select end
        var clip: vec4<f32>;
        if ( position.y < 0.5 ) {{
          clip = clipStart;
        }} else {{
          clip = clipEnd;
        }}

        // back to clip space
        offset = offset * clip.w;
        clip = vec4<f32>(clip.xy + offset, clip.zw);

        out.position = clip;
        out.uv = uv;
        out.color = fatline_color;

        return out;
      }}
    "))
      .use_vertex_entry("vs_fatline_main")
      .include_fragment_entry("
        [[stage(fragment)]]
        fn fs_main(in: FatlineVertexOutput) -> [[location(0)]] vec4<f32> {

          // discard corner
          let vUv = in.uv;
          if ( abs( vUv.y ) > 1.0 ) {
            let a = vUv.x;
            var b: f32;
            if ( vUv.y > 0.0 ) {
              b = vUv.y - 1.0;
            } else {
              b = vUv.y + 1.0;
            }
            let len2 = a * a + b * b;
            if ( len2 > 1.0 ) {
              discard;
            }
          }

          return in.color;
        }
        ")
      .use_fragment_entry("fs_main")
      .declare_io_struct(
        "
      struct FatlineVertexOutput {
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] uv: vec2<f32>;
        [[location(1)]] color: vec4<f32>;
      };
    ",
      );

    builder
      .with_layout::<FatLineMaterial>(ctx.layouts, device)
      .with_layout::<PassGPUData>(ctx.layouts, device);

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
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
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
