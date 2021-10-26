use std::borrow::Cow;

use rendiation_algebra::Vec3;
use rendiation_algebra::Vector;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use rendiation_renderable_mesh::tessellation::IndexedMeshTessellator;
use rendiation_renderable_mesh::tessellation::SphereMeshParameter;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_renderable_mesh::GPUMeshData;
use rendiation_webgpu::*;

use crate::*;

pub trait Background: 'static + SceneRenderable {
  fn require_pass_clear(&self) -> Option<wgpu::Color>;
}

pub struct SolidBackground {
  pub intensity: Vec3<f32>,
}

impl Background for SolidBackground {
  fn require_pass_clear(&self) -> Option<wgpu::Color> {
    wgpu::Color {
      r: self.intensity.r() as f64,
      g: self.intensity.g() as f64,
      b: self.intensity.b() as f64,
      a: 1.,
    }
    .into()
  }
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      intensity: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl SolidBackground {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

impl SceneRenderable for SolidBackground {
  fn update(
    &mut self,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtxBase,
    _components: &mut SceneComponents,
  ) {
  }

  fn setup_pass<'a>(
    &self,
    _pass: &mut GPURenderPass<'a>,
    _components: &SceneComponents,
    _camera_gpu: &CameraBindgroup,
    _pipeline_resource: &GPUResourceCache,
    _pass_info: &PassTargetFormatInfo,
  ) {
  }
}

pub type BackgroundMesh = impl GPUMeshData;
fn build_mesh() -> BackgroundMesh {
  let sphere = SphereMeshParameter {
    radius: 100.,
    ..Default::default()
  };
  sphere.tessellate()
}
use crate::scene::mesh::Mesh;

pub struct DrawableBackground<S: MaterialCPUResource> {
  mesh: MeshCellInner<BackgroundMesh>,
  pub shading: MaterialCell<S>,
}

impl<S> Background for DrawableBackground<S>
where
  S: MaterialCPUResource + 'static,
  MaterialCell<S>: materials::Material,
{
  fn require_pass_clear(&self) -> Option<wgpu::Color> {
    None
  }
}

impl<S> SceneRenderable for DrawableBackground<S>
where
  S: MaterialCPUResource,
  MaterialCell<S>: materials::Material,
{
  fn update(
    &mut self,
    gpu: &GPU,
    base: &mut SceneMaterialRenderPrepareCtxBase,
    components: &mut SceneComponents,
  ) {
    components
      .nodes
      .borrow_mut()
      .get_root_node_mut()
      .data_mut()
      .get_model_gpu(gpu);

    self.mesh.update(gpu, &mut base.resources.custom_storage);

    let mut ctx = SceneMaterialRenderPrepareCtx {
      base,
      model_info: None,
      active_mesh: None,
    };
    self.shading.update(gpu, &mut ctx);
  }

  fn setup_pass<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    components: &SceneComponents,
    camera_gpu: &CameraBindgroup,
    resources: &GPUResourceCache,
    pass_info: &PassTargetFormatInfo,
  ) {
    let nodes = components.nodes.borrow();
    let ctx = SceneMaterialPassSetupCtx {
      pass: pass_info,
      camera_gpu,
      model_gpu: nodes.get_root_node().data().gpu.as_ref().unwrap().into(),
      resources,
      active_mesh: None,
    };
    self.shading.setup_pass(pass, &ctx);
    self.mesh.setup_pass_and_draw(pass, MeshDrawGroup::Full);
  }
}

impl<S: BackGroundShading> DrawableBackground<S> {
  pub fn new(shading: MaterialCell<S>) -> Self {
    let mesh = build_mesh();
    let mesh = MeshCellInner::new(mesh);

    Self { mesh, shading }
  }
}

pub trait BackGroundShading: MaterialCPUResource {
  fn shader_header(&self) -> &'static str;

  fn shading(&self) -> &'static str;

  fn shader(&self) -> String {
    format!(
      "
    {object_header}
    {material_header}
    {camera_header}

    {background_shading}

    struct VertexOutput {{
      [[builtin(position)]] position: vec4<f32>;
      [[location(0)]] uv: vec2<f32>;
      [[location(1)]] world_position: vec3<f32>;
    }};

    [[stage(vertex)]]
    fn vs_main(
      {vertex_header}
    ) -> VertexOutput {{
      var out: VertexOutput;
      out.uv = uv;
      out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);
      out.position.z = out.position.w;
      out.world_position = (model.matrix * vec4<f32>(position, 1.0)).xyz;
      return out;
    }}

    [[stage(fragment)]]
    fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
      let direction = normalize(in.world_position);
      return vec4<f32>(background_shading(direction), 1.0);
    }}
    ",
      vertex_header = Vertex::get_shader_header(),
      material_header = self.shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = TransformGPU::get_shader_header(),
      background_shading = self.shading()
    )
  }

  fn create_bindgroup_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout;

  fn create_pipeline(
    &self,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline {
    let bindgroup_layout = self.create_bindgroup_layout(device);
    let shader_source = self.shader();

    let states = MaterialStates {
      depth_write_enabled: false,
      depth_compare: wgpu::CompareFunction::Always,
      ..Default::default()
    };

    println!("{}", shader_source);

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[
        ctx.layouts.retrieve::<TransformGPU>(device),
        &bindgroup_layout,
        ctx.layouts.retrieve::<CameraBindgroup>(device),
      ],
      push_constant_ranges: &[],
    });

    let vertex_buffers = vec![Vertex::vertex_layout()];

    let targets: Vec<_> = ctx
      .pass
      .color_formats
      .iter()
      .map(|&f| states.map_color_states(f))
      .collect();

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        targets: targets.as_slice(),
      }),
      primitive: wgpu::PrimitiveState {
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: states.map_depth_stencil_state(ctx.pass.depth_stencil_format),
      multisample: wgpu::MultisampleState::default(),
    })
  }
}
