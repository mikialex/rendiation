use rendiation_scene_webgpu::{generate_quad, CameraGPU};
use shadergraph::*;
use webgpu::UniformBufferDataView;
use wgsl_shader_derives::wgsl_fn;

// http://asliceofrendering.com/scene%20helper/2020/01/05/InfiniteGrid/

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct InfinityShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

pub struct InfinityShaderPlaneEffect<'a> {
  plane: UniformBufferDataView<InfinityShaderPlane>,
  camera: &'a CameraGPU,
}

impl<'a> ShaderGraphProvider for InfinityShaderPlaneEffect<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      let out = generate_quad(builder.vertex_index).expand();
      builder.register::<WorldVertexPosition>(out.position.xyz()); // too feed camera needs
      Ok(())
    })?;

    self.camera.build(builder)?;

    builder.vertex(|builder, _| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      let position = builder.query::<WorldVertexPosition>()?;
      let position_xy = position.xy();

      let near = unproject_point(position_xy, 0., view, proj);
      let far = unproject_point(position_xy, 1., view, proj);

      builder.set_vertex_out::<InfinityNear>(near);
      builder.set_vertex_out::<InfinityFar>(far);

      builder.register::<ClipPosition>((position, 1.));
      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;

      let near = builder.query::<InfinityNear>()?;
      let far = builder.query::<InfinityNear>()?;
      let plane = binding.uniform_by(&self.plane, SB::Object).expand();

      // todo test near-far line seg hit plane

      let plane_hit: Vec3<f32> = todo!();
      let plane_hit_project = proj * view * (plane_hit, consts(1.)).into();
      builder.set_explicit_depth(plane_hit_project.z() / plane_hit_project.w());

      builder.register::<FragmentWorldPosition>(plane_hit);
      Ok(())
    })
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      // check if visible and clear the alpha channel;
      builder.register::<FragmentAlpha>(node);
      Ok(())
    })
  }
}

both!(InfinityNear, Vec3<f32>);
both!(InfinityFar, Vec3<f32>);

both!(CameraRayDirection, Vec3<f32>);

wgsl_fn! {
  fn unproject_point(xy: vec2<f32>, z: f32, view: mat4x4<f32>, projection: mat4x4<f32>) -> vec3<f32> {
    let unprojected =  inverse(view) * inverse(projection) * vec4<f32>(x, y, z, 1.0);
    return unprojected.xyz / unprojected.w;
  }
}
