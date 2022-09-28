use rendiation_scene_webgpu::{generate_quad, CameraGPU};
use shadergraph::*;
use webgpu::UniformBufferDataView;
use wgsl_shader_derives::wgsl_fn;

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
      builder.register::<FragmentUv>(out.uv);
      builder.register::<WorldVertexPosition>(out.position.xyz()); // too feed camera needs
      Ok(())
    })?;

    self.camera.build(builder)?;

    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      let view_inv = builder.query::<CameraWorldMatrix>()?;
      let uv = builder.query::<FragmentUv>()?;
      let plane = binding.uniform_by(&self.plane, SB::Object);

      let unprojected =
        view_inv * proj.inverse() * (uv * consts(2.) - consts(Vec2::one()), 0., 1.).into();
      let unprojected = unprojected.xyz() / unprojected.w();

      let origin = view_inv.position();
      let direction = (unprojected - origin).normalize();

      let hit = ray_plane_intersect(origin, direction, plane);

      let plane_hit = hit.xyz();
      let plane_if_hit = hit.w(); // 1 is hit, 0 is not

      let plane_hit_project = proj * view * (plane_hit, consts(1.)).into();
      builder.set_explicit_depth(plane_hit_project.z() / plane_hit_project.w());

      builder.register::<FragmentWorldPosition>(plane_hit);
      builder.register::<IsHitInfinityPlane>(plane_if_hit);
      Ok(())
    })
  }

  // override
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      let has_hit = builder.query::<IsHitInfinityPlane>()?;
      builder.register::<FragmentAlpha>(has_hit);
      Ok(())
    })
  }
}

both!(IsHitInfinityPlane, f32);

wgsl_fn! {
  fn ray_plane_intersect(origin: vec3<f32>, direction: vec3<f32>, plane: InfinityShaderPlane) -> vec4<f32> {
    let denominator = dot(plane.normal, direction);

    // if denominator == T::zero() {
    //   // line is coplanar, return origin
    //   if plane.distance_to(&self.origin) == T::zero() {
    //     return T::zero().into();
    //   }

    //   return None;
    // }

    let t = -(dot(origin, plane.normal) + plane.constant) / denominator;

    if (t >= 0.0) {
      return vec4<f32>(origin + direction * t, 1.0);
    } else {
      return vec4<f32>(0.0);
    }
  }
}
