use rendiation_geometry::*;
use rendiation_lighting_gpu_system::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod lighting;
use lighting::*;
mod octahedral;
use octahedral::*;

pub struct ProbeVolume {
  spacing: f32, // in meter, world space
  bound: Box3,
  sample_width: u32, // recommended width: 4
}

impl ProbeVolume {
  pub fn probe_instance_count(&self) -> u32 {
    let size = self.bound.size();
    let count = (size.x / self.spacing).ceil()
      * (size.y / self.spacing).ceil()
      * (size.z / self.spacing).ceil();
    count as u32
  }
  pub fn per_probe_sample_count(&self) -> u32 {
    self.sample_width * self.sample_width * 4
  }

  /// take padding into count
  pub fn per_probe_texel_storage_requirement(&self) -> u32 {
    self.per_probe_sample_count() + self.sample_width * 2 * 4 + 4
  }

  pub fn texel_storage_requirement(&self) -> u32 {
    self.probe_instance_count() * self.per_probe_texel_storage_requirement()
  }

  pub fn create_gpu_instance(&self, gpu: &GPU) -> ProbeVolumeGPUInstance {
    todo!()
  }
}

#[derive(Debug, Clone, ShaderStruct)]
pub struct ProbeVolumeGPUInfo {
  /// volume center location in world_space
  pub origin: Vec3<f32>,
  /// world-space distance between probes
  pub spacing: Vec3<f32>,
  /// number of probes on each axis of the volume
  pub counts: Vec3<u32>,

  /// grid-space offsets used for scrolling movement
  pub scroll_offsets: Vec3<i32>,
  /// whether probes of a plane need to be cleared due to scrolling movement
  pub scroll_clear: Vec3<u32>, // Vec3<bool>
  /// direction of scrolling movement (0: negative, 1: positive)
  pub scroll_direction: Vec3<u32>,
}

pub struct ProbeVolumeGPUInstance {
  // todo
}

pub trait DDGISceneBridge {}

impl ProbeVolumeGPUInstance {
  pub fn update(frame: &mut FrameCtx, scene: &dyn DDGISceneBridge) {
    // per probe trace new ray to get gbuffer

    // do gbuffer direct lighting use defer pipeline

    // blend the direct lighting result into probe buffer
  }

  pub fn create_lighting_component(&self) -> Box<dyn LightingComputeComponent> {
    // use probe buffer for indirect lighting
    todo!()
  }
}
