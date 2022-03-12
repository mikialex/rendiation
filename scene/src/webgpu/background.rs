// use rendiation_renderable_mesh::group::MeshDrawGroup;
// use rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh;
// use rendiation_renderable_mesh::tessellation::IndexedMeshTessellator;
// use rendiation_renderable_mesh::tessellation::SphereMeshParameter;
// use rendiation_renderable_mesh::vertex::Vertex;
// use rendiation_renderable_mesh::GPUMeshData;
use rendiation_webgpu::*;

use crate::*;

pub trait WebGPUBackground: 'static + SceneRenderable {
  fn require_pass_clear(&self) -> Option<wgpu::Color>;
}

impl WebGPUBackground for SolidBackground {
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

impl SceneRenderable for SolidBackground {
  fn setup_pass<'a>(
    &self,
    _gpu: &GPU,
    _pass: &mut SceneRenderPass,
    _dispatcher: &dyn SourceOfRendering,
    _camera: &SceneCamera,
  ) {
  }
}

// pub type BackgroundMesh = impl GPUMeshData + IntersectAbleGroupedMesh;
// fn build_mesh() -> BackgroundMesh {
//   let sphere = SphereMeshParameter {
//     radius: 100.,
//     ..Default::default()
//   };
//   sphere.tessellate()
// }

// pub struct DrawableBackground<S: WebGPUMaterial> {
//   mesh: MeshInner<MeshSource<BackgroundMesh>>,
//   pub shading: MaterialInner<S>,
//   root: SceneNode,
// }

// impl<S> Background for DrawableBackground<S>
// where
//   S: WebGPUMaterial,
// {
//   fn require_pass_clear(&self) -> Option<wgpu::Color> {
//     None
//   }
// }

// impl<S> SceneRenderable for DrawableBackground<S>
// where
//   S: WebGPUMaterial,
// {
//   fn setup_pass<'a>(
//     &self,
//     gpu: &GPU,
//     pass: &mut SceneRenderPass<'a>,
//     camera_gpu: &CameraBindgroup,
//     resources: &mut GPUResourceCache,
//   ) {
//     self.root.visit(|node| {
//       let model_gpu = resources.content.nodes.check_update_gpu(node);
//       let ctx = SceneMaterialPassSetupCtx {
//         camera_gpu,
//         model_gpu,
//         resources: &resources.content,
//       };
//       resources.scene.setup_material(&self.shading, pass, &ctx);
//       self
//         .mesh
//         .setup_pass_and_draw(pass, MeshDrawGroup::Full, &resources.scene);
//     });
//   }
// }

// impl<S: BackGroundShading> DrawableBackground<S> {
//   pub fn new(shading: MaterialInner<S>, root: SceneNode) -> Self {
//     let mesh = build_mesh();
//     let mesh = MeshInner::new(MeshSource::new(mesh));

//     Self {
//       mesh,
//       shading,
//       root,
//     }
//   }
// }

// pub trait BackGroundShading: WebGPUMaterial + BindGroupLayoutProvider {
//   fn shading(&self) -> &'static str;

//   fn shader(&self, builder: &mut PipelineBuilder) {
//     builder
//       .include(self.shading())
//       .declare_io_struct(
//         "
//      struct VertexOutputBackground {{
//       [[builtin(position)]] position: vec4<f32>;
//       [[location(0)]] uv: vec2<f32>;
//       [[location(1)]] world_position: vec3<f32>;
//     }};
//     ",
//       )
//       .include_vertex_entry(
//         "
//       [[stage(vertex)]]
//       fn vs_main(
//         [[location(0)]] position: vec3<f32>, // todo link with vertex type
//         [[location(1)]] normal: vec3<f32>,
//         [[location(2)]] uv: vec2<f32>,
//       ) -> VertexOutputBackground {{
//         var out: VertexOutput;
//         out.uv = uv;
//         out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);
//         out.position.z = out.position.w;
//         out.world_position = (model.matrix * vec4<f32>(position, 1.0)).xyz;
//         return out;
//       }}

//     ",
//       )
//       .use_vertex_entry("vs_main")
//       .include_vertex_entry(
//         "
//         [[stage(fragment)]]
//         fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
//           let direction = normalize(in.world_position);
//           return vec4<f32>(background_shading(direction), 1.0);
//         }}
//     ",
//       )
//       .use_vertex_entry("fs_main");
//   }

//   fn create_pipeline(
//     &self,
//     builder: &mut PipelineBuilder,
//     device: &wgpu::Device,
//     ctx: &PipelineCreateCtx,
//   ) {
//     let states = MaterialStates {
//       depth_write_enabled: false,
//       depth_compare: wgpu::CompareFunction::Always,
//       ..Default::default()
//     };

//     self.shader(builder);

//     builder
//       .with_layout::<TransformGPU>(ctx.layouts, device)
//       .with_layout::<Self>(ctx.layouts, device)
//       .with_layout::<CameraBindgroup>(ctx.layouts, device);

//     builder.vertex_buffers = vec![Vertex::vertex_layout()];

//     builder.targets = ctx
//       .pass_info
//       .format_info
//       .color_formats
//       .iter()
//       .map(|&f| states.map_color_states(f))
//       .collect();

//     builder.depth_stencil =
//       states.map_depth_stencil_state(ctx.pass_info.format_info.depth_stencil_format);
//   }
// }
