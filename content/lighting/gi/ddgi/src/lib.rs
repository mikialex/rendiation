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

  /// type of movement the volume allows. 0: default, 1: infinite scrolling
  pub movementType: u32,

  /// number of rays traced per probe
  pub num_rays: u32,
  /// number of texels in one dimension of a probe's irradiance texture (does not include 1-texel border)
  pub numIrradianceInteriorTexels: u32,
  /// number of texels in one dimension of a probe's distance texture (does not include 1-texel border)
  pub numDistanceInteriorTexels: u32,

  /// weight of the previous irradiance and distance data store in probes
  pub probeHysteresis: f32,
  /// maximum world-space distance a probe ray can travel
  pub probeMaxRayDistance: f32,
  /// offset along the surface normal, applied during lighting to avoid numerical instabilities when determining visibility                
  pub probeNormalBias: f32,
  ///  offset along the camera view ray, applied during lighting to avoid numerical instabilities when determining visibility
  pub probeViewBias: f32,
  /// exponent used during visibility testing. High values react rapidly to depth discontinuities, but may cause banding                     
  pub probeDistanceExponent: f32,
  /// exponent that perceptually encodes irradiance for faster light-to-dark convergence
  pub probeIrradianceEncodingGamma: f32,

  /// threshold to identify when large lighting changes occur
  pub probeIrradianceThreshold: f32,
  /// threshold that specifies the maximum allowed difference in brightness between the previous and current irradiance values
  pub probeBrightnessThreshold: f32,
  /// threshold that specifies the ratio of *random* rays traced for a probe that may hit back facing triangles before the probe is considered inside geometry (used in blending)
  pub probeRandomRayBackfaceThreshold: f32,

  // Probe Relocation, Probe Classification
  /// threshold that specifies the ratio of *fixed* rays traced for a probe that may hit back facing triangles before the probe is considered inside geometry (used in relocation & classification)
  pub probeFixedRayBackfaceThreshold: f32,
  /// minimum world-space distance to a front facing triangle allowed before a probe is relocated
  pub probeMinFrontfaceDistance: f32,

  // Infinite Scrolling Volumes
  /// grid-space offsets used for scrolling movement
  pub scroll_offsets: Vec3<i32>,
  /// whether probes of a plane need to be cleared due to scrolling movement
  pub scroll_clear: Vec3<u32>, // Vec3<bool>
  /// direction of scrolling movement (0: negative, 1: positive)
  pub scroll_direction: Vec3<u32>,

  // Feature Options
  /// texture format of the ray data texture (EDDGIVolumeTextureFormat)
  pub probeRayDataFormat: u32,
  /// texture format of the irradiance texture (EDDGIVolumeTextureFormat)
  pub probeIrradianceFormat: u32,
  /// whether probe relocation is enabled for this volume
  pub probeRelocationEnabled: Bool,
  /// whether probe classification is enabled for this volume
  pub probeClassificationEnabled: Bool,
  /// whether probe variability is enabled for this volume
  pub probeVariabilityEnabled: Bool,
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
