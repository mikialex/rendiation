use rendiation_shader_library::{
  sampling::concentric_sample_disk_device_fn, shader_uv_space_to_world_space,
};

use crate::*;

pub trait RtxCameraRenderImpl {
  fn get_rtx_camera(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RtxCameraRenderComponent>;
}

impl RtxCameraRenderImpl for CameraRenderer {
  fn get_rtx_camera(
    &self,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> Box<dyn RtxCameraRenderComponent> {
    Box::new(CameraGPU {
      ubo: self.0.get(&camera).unwrap().clone(),
    })
  }
}

pub trait RtxCameraRenderComponent: ShaderHashProvider + DynClone {
  fn build_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RtxCameraRenderInvocation>;
  fn bind(&self, binding: &mut BindingBuilder);
}
clone_trait_object!(RtxCameraRenderComponent);

impl RtxCameraRenderComponent for CameraGPU {
  fn build_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RtxCameraRenderInvocation> {
    Box::new(DefaultRtxCameraInvocation {
      camera: binding.bind_by(&self.ubo),
    })
  }

  fn bind(&self, binding: &mut BindingBuilder) {
    binding.bind(&self.ubo);
  }
}

pub trait RtxCameraRenderInvocation: DynClone {
  fn generate_ray(
    &self,
    pixel_index: Node<Vec2<u32>>,
    frame_size: Node<Vec2<u32>>,
    sampler: &dyn DeviceSampler,
  ) -> ShaderRay;
}

clone_trait_object!(RtxCameraRenderInvocation);

#[derive(Clone)]
pub struct DefaultRtxCameraInvocation {
  camera: ShaderReadonlyPtrOf<CameraGPUTransform>,
}

impl RtxCameraRenderInvocation for DefaultRtxCameraInvocation {
  fn generate_ray(
    &self,
    pixel_index: Node<Vec2<u32>>,
    frame_size: Node<Vec2<u32>>,
    sampler: &dyn DeviceSampler,
  ) -> ShaderRay {
    let uv = pixel_index.into_f32() / frame_size.into_f32();
    let uv = uv + sampler.next_2d() / frame_size.into_f32();

    // let focal_length = val(0.035); // match our camera default 50 horizontal fov
    // let aperture = val(1.4);
    // let test_camera_info = ENode::<PhysicalCameraGPUInfo> {
    //   lens_radius: focal_length / (aperture * val(2.0)),
    //   focal_length,
    //   focal_plane_distance: val(2.0),
    //   sensor_size: val(Vec2::new(0.035, 0.024)), // full frame sensor
    // };

    // let uv = uv * val(2.) - val(Vec2::splat(1.));
    // let local_ray = compute_local_ray_in_camera_space(test_camera_info, sampler, uv);
    // local_ray.transform(self.camera.world().load())

    let view_projection_inv = self.camera.view_projection_inv().load();
    let world = self.camera.world().load();

    let world_target = shader_uv_space_to_world_space(view_projection_inv, uv, val(1.));

    let origin = world.position();

    let direction = (world_target - origin).normalize();

    ShaderRay { origin, direction }
  }
}

#[derive(Clone, Copy, ShaderStruct)]
pub struct PhysicalCameraGPUInfo {
  pub lens_radius: f32,
  pub focal_length: f32,
  pub focal_plane_distance: f32,
  pub sensor_size: Vec2<f32>,
}

pub fn compute_local_ray_in_camera_space(
  camera: ENode<PhysicalCameraGPUInfo>,
  sampler: &dyn DeviceSampler,
  normalized_film_sample_position: Node<Vec2<f32>>, // -1. to 1.
) -> ShaderRay {
  // sample a point on lens
  let position_on_lens = concentric_sample_disk_device_fn(sampler.next_2d()) * camera.lens_radius;

  // compute point on focus plane
  let ratio = camera.focal_plane_distance / camera.focal_length;
  let position_on_film = camera.sensor_size * val(0.5) * normalized_film_sample_position;
  let scaled = position_on_film * ratio;
  let focus_point_on_focus_plane: Node<Vec3<_>> =
    (scaled.x(), -scaled.y(), -camera.focal_plane_distance).into();

  let origin: Node<Vec3<_>> = (position_on_lens.x(), position_on_lens.y(), val(0.)).into();
  ShaderRay {
    origin,
    direction: (focus_point_on_focus_plane - origin).normalize(),
  }
}
