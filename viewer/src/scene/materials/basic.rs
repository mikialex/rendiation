use std::borrow::Cow;

use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;

use crate::{
  renderer::Renderer,
  scene::{
    CameraBindgroup, ModelTransformGPU, SamplerHandle, StandardForward, Texture2DHandle,
    VertexBufferSourceType,
  },
};

use super::{
  BindableResource, MaterialCPUResource, MaterialGPUResource, SceneMaterialPassSetupCtx,
  SceneMaterialRenderPrepareCtx,
};

pub struct BasicMaterial {
  pub color: Vec3<f32>,
  pub sampler: SamplerHandle,
  pub texture: Texture2DHandle,
}

impl BasicMaterial {
  pub fn create_bindgroup<S>(
    &self,
    ubo: &wgpu::Buffer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: ubo.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: ctx
            .textures
            .get_mut(self.texture)
            .unwrap()
            .get_gpu(device, queue)
            .as_bindable(),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: ctx
            .samplers
            .get_mut(self.sampler)
            .unwrap()
            .get_gpu(device)
            .as_bindable(),
        },
      ],
      label: None,
    })
  }

  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct BasicMaterial {
      color: vec3<f32>;
    };

    [[group(1), binding(0)]]
    var basic_material: BasicMaterial;
    
    [[group(1), binding(1)]]
    var r_color: texture_2d<f32>;

    [[group(1), binding(2)]]
    var r_sampler: sampler;
    "
  }

  pub fn create_bindgroup_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStage::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Vec3<f32>>() as u64),
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            multisampled: false,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: wgpu::BindingType::Sampler {
            comparison: false,
            filtering: true,
          },
          count: None,
        },
      ],
    })
  }
}

pub struct BasicMaterialGPU {
  uniform: wgpu::Buffer,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: wgpu::BindGroup,
}

impl MaterialGPUResource<StandardForward> for BasicMaterialGPU {
  type Source = BasicMaterial;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<StandardForward>,
  ) {
    //
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, StandardForward>,
  ) {
    let pipeline = ctx.pipelines.basic.as_ref().unwrap();
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &ctx.model_gpu.bindgroup, &[]);
    pass.set_bind_group(1, &self.bindgroup, &[]);
    pass.set_bind_group(2, &ctx.camera_gpu.bindgroup, &[]);
  }
}

impl MaterialCPUResource for BasicMaterial {
  type GPU = BasicMaterialGPU;

  fn create<S>(
    &mut self,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU {
    use wgpu::util::DeviceExt;
    let uniform: wgpu::Buffer =
      renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("Basic Material Uniform Buffer"),
          contents: bytemuck::cast_slice(&[self.color]),
          usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

    let bindgroup_layout = Self::create_bindgroup_layout(&renderer.device);
    let bindgroup = self.create_bindgroup(
      &uniform,
      &renderer.device,
      &renderer.queue,
      &bindgroup_layout,
      ctx,
    );

    let shader_source = format!(
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
        out.uv = uv;
        out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);;
        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return textureSample(r_color, r_sampler, in.uv);
      }}
      
      ",
      vertex_header = Vertex::get_shader_header(),
      material_header = Self::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = ModelTransformGPU::get_shader_header(),
    );

    let shader = renderer
      .device
      .create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
        flags: renderer.create_shader_flags(),
      });

    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
          &ctx.model_gpu.layout,
          &bindgroup_layout,
          &ctx.camera_gpu.layout,
        ],
        push_constant_ranges: &[],
      });

    let vertex_buffers = [Vertex::get_layout()];
    let pipeline = renderer
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader,
          entry_point: "vs_main",
          buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader,
          entry_point: "fs_main",
          targets: &[renderer.get_prefer_target_format().into()],
        }),
        // primitive: wgpu::PrimitiveState {
        //   cull_mode: wgpu::Face::Back.into(),
        //   ..Default::default()
        // },
        primitive: wgpu::PrimitiveState {
          cull_mode: None,
          topology: wgpu::PrimitiveTopology::TriangleList,
          ..Default::default()
        },
        depth_stencil: wgpu::DepthStencilState {
          format: StandardForward::depth_format(),
          depth_write_enabled: true,
          depth_compare: wgpu::CompareFunction::Less,
          stencil: Default::default(),
          bias: Default::default(),
        }
        .into(),
        multisample: wgpu::MultisampleState::default(),
      });

    ctx.pipelines.basic = pipeline.into();

    BasicMaterialGPU {
      uniform,
      bindgroup_layout,
      bindgroup,
    }
  }
}

struct RenderPipelineBuilder {
  primitive: wgpu::PrimitiveState,
  depth_stencil: Option<wgpu::DepthStencilState>,
  multisample: wgpu::MultisampleState,
}
