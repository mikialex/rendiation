use rendiation_geometry::*;
use rendiation_lighting_gpu_system::*;
use rendiation_webgpu::*;

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

pub struct ProbeVolumeGPUInstance {
  // todo
}

pub trait DDGISceneBridge {}

impl ProbeVolumeGPUInstance {
  pub fn update(frame: &mut FrameCtx, scene: &dyn DDGISceneBridge) {
    // todo
  }

  pub fn create_lighting_component(&self) -> Box<dyn LightingComputeComponent> {
    todo!()
  }
}
