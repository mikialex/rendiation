//! The wgpu api call count statistics.
//!
//! Only the calls that actually reach wgpu through the wrapper types in this crate are counted. The
//! calls made through the raw access methods (for example [GPUDevice::raw_device]) or through the
//! wgpu objects cloned out of the wrapper are not counted.
//!
//! The command recording calls are counted by plain integers in each pass and encoder. A pass merges
//! its counts into the encoder when dropped, and the encoder's counts are merged into the device when
//! submitted. So the counts in an encoder are the recorded counts, and the counts in the device
//! report are the submitted counts. The device and queue level calls are counted by relaxed atomics.

use std::sync::atomic::AtomicU64;

use crate::*;

/// The snapshot of the device level call counts. The counts are monotonically increased, to get the
/// counts in a period (for example a frame), diff the snapshots by [GPUCallCountReport::delta_since].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GPUCallCountReport {
  pub device: GPUDeviceCallCounts,
  pub submitted_commands: GPUCommandCallCounts,
}

impl GPUCallCountReport {
  pub fn delta_since(&self, previous: &Self) -> Self {
    Self {
      device: self.device.delta_since(&previous.device),
      submitted_commands: self
        .submitted_commands
        .delta_since(&previous.submitted_commands),
    }
  }
}

impl GPUDevice {
  pub fn create_call_count_report(&self) -> GPUCallCountReport {
    self.inner.counters.report()
  }
}

impl GPU {
  pub fn create_call_count_report(&self) -> GPUCallCountReport {
    self.device.create_call_count_report()
  }
}

define_call_counts! {
  /// The device and queue level calls
  GPUDeviceCallCounts, AtomicGPUDeviceCallCounts {
    create_shader_module,
    create_command_encoder,
    create_bind_group,
    create_bind_group_layout,
    create_pipeline_layout,
    create_render_pipeline,
    create_mesh_pipeline,
    create_compute_pipeline,
    create_pipeline_cache,
    create_buffer,
    create_buffer_bytes,
    /// the initial data size of [GPUDevice::create_buffer_init]
    create_buffer_init_bytes,
    create_texture,
    create_texture_view,
    create_external_texture,
    create_sampler,
    create_query_set,
    create_blas,
    create_tlas,
    create_render_bundle_encoder,
    device_poll,
    /// only happens in web target, see [DeferExplicitDestroy]
    explicit_destroy,
    buffer_map_async,
    buffer_map_async_bytes,
    buffer_get_mapped_range,
    buffer_unmap,
    queue_submit,
    queue_submit_command_buffers,
    queue_write_buffer,
    queue_write_buffer_bytes,
    queue_write_buffer_with,
    queue_write_buffer_with_bytes,
    queue_write_texture,
    queue_write_texture_bytes,
    queue_copy_external_image_to_texture,
    queue_compact_blas,
    surface_configure,
    surface_get_current_texture,
    surface_present,
  }
}

define_call_counts! {
  /// The command recording calls in encoder and passes
  GPUCommandCallCounts, AtomicGPUCommandCallCounts {
    begin_render_pass,
    begin_compute_pass,
    set_pipeline,
    set_bind_group,
    set_vertex_buffer,
    set_index_buffer,
    set_viewport,
    set_scissor_rect,
    set_stencil_reference,
    set_blend_constant,
    set_immediates,
    set_immediates_bytes,
    draw,
    draw_indexed,
    draw_indirect,
    draw_indexed_indirect,
    multi_draw_indirect,
    multi_draw_indexed_indirect,
    multi_draw_indirect_count,
    multi_draw_indexed_indirect_count,
    draw_mesh_tasks,
    draw_mesh_tasks_indirect,
    multi_draw_mesh_tasks_indirect,
    multi_draw_mesh_tasks_indirect_count,
    /// the sum of the count(or max count) parameter of all multi draw calls, the actual draw count
    /// of the indirect count variant is decided by gpu.
    multi_draw_max_count,
    execute_bundles,
    dispatch_workgroups,
    dispatch_workgroups_indirect,
    copy_buffer_to_buffer,
    copy_buffer_to_buffer_bytes,
    copy_buffer_to_texture,
    copy_buffer_to_texture_bytes,
    copy_texture_to_buffer,
    copy_texture_to_buffer_bytes,
    copy_texture_to_texture,
    copy_texture_to_texture_bytes,
    clear_buffer,
    clear_buffer_bytes,
    clear_texture,
    resolve_query_set,
    write_timestamp,
    begin_occlusion_query,
    end_occlusion_query,
    begin_pipeline_statistics_query,
    end_pipeline_statistics_query,
    build_acceleration_structures,
    build_blas_entries,
    build_tlas_entries,
    mark_acceleration_structures_built,
    transition_resources,
    map_buffer_on_submit,
    on_submitted_work_done,
    /// insert_debug_marker, push_debug_group and pop_debug_group
    debug_marker,
  }
}

/// The shared counters of a device, also hold by the queue and resources. It's opaque outside, use
/// [GPUDevice::create_call_count_report] to read the counts.
#[derive(Default)]
pub struct GPUCallCounters {
  pub(crate) device: AtomicGPUDeviceCallCounts,
  pub(crate) submitted_commands: AtomicGPUCommandCallCounts,
}

impl GPUCallCounters {
  fn report(&self) -> GPUCallCountReport {
    GPUCallCountReport {
      device: self.device.load(),
      submitted_commands: self.submitted_commands.load(),
    }
  }
}

/// the pass merges its counts into this sink when dropped, the sink is owned by the encoder
pub(crate) type PassCallCountSink = Arc<parking_lot::Mutex<GPUCommandCallCounts>>;

#[derive(Default)]
pub(crate) struct AtomicCallCount(AtomicU64);

impl AtomicCallCount {
  #[inline(always)]
  pub fn inc(&self) {
    self.add(1);
  }
  #[inline(always)]
  pub fn add(&self, count: u64) {
    self.0.fetch_add(count, Ordering::Relaxed);
  }
  fn load(&self) -> u64 {
    self.0.load(Ordering::Relaxed)
  }
}

/// the byte size of a texel copy, the block compressed format is considered. return 0 if the format
/// and aspect combination is not copyable.
pub(crate) fn texel_copy_byte_size(
  format: gpu::TextureFormat,
  aspect: gpu::TextureAspect,
  size: gpu::Extent3d,
) -> u64 {
  let Some(block_size) = format.block_copy_size(Some(aspect)) else {
    return 0;
  };
  let (block_width, block_height) = format.block_dimensions();
  let width_blocks = size.width.div_ceil(block_width) as u64;
  let height_blocks = size.height.div_ceil(block_height) as u64;
  width_blocks * height_blocks * size.depth_or_array_layers as u64 * block_size as u64
}

macro_rules! define_call_counts {
  (
    $(#[$meta:meta])*
    $name:ident, $atomic_name:ident {
      $($(#[$field_meta:meta])* $field:ident,)*
    }
  ) => {
    $(#[$meta])*
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct $name {
      $($(#[$field_meta])* pub $field: u64,)*
    }

    impl $name {
      /// visit all counts by name, the name is same as the field name
      pub fn for_each_count(&self, mut visitor: impl FnMut(&'static str, u64)) {
        $(visitor(stringify!($field), self.$field);)*
      }

      pub fn delta_since(&self, previous: &Self) -> Self {
        Self {
          $($field: self.$field.saturating_sub(previous.$field),)*
        }
      }
    }

    impl std::ops::AddAssign<&$name> for $name {
      fn add_assign(&mut self, other: &$name) {
        $(self.$field += other.$field;)*
      }
    }

    #[derive(Default)]
    pub(crate) struct $atomic_name {
      $(pub $field: AtomicCallCount,)*
    }

    impl $atomic_name {
      #[allow(dead_code)]
      pub fn merge(&self, counts: &$name) {
        $(
          if counts.$field != 0 {
            self.$field.add(counts.$field);
          }
        )*
      }

      fn load(&self) -> $name {
        $name {
          $($field: self.$field.load(),)*
        }
      }
    }
  };
}
use define_call_counts;

#[test]
fn test_texel_copy_byte_size() {
  let size = |width, height| gpu::Extent3d {
    width,
    height,
    depth_or_array_layers: 2,
  };
  let all = gpu::TextureAspect::All;
  assert_eq!(
    texel_copy_byte_size(gpu::TextureFormat::Rgba8Unorm, all, size(64, 64)),
    64 * 64 * 4 * 2
  );
  // the block is 4x4 with 8 bytes, the partial block is counted as a full block
  assert_eq!(
    texel_copy_byte_size(gpu::TextureFormat::Bc1RgbaUnorm, all, size(10, 10)),
    3 * 3 * 8 * 2
  );
  // the combined depth stencil format requires a single aspect to copy
  assert_eq!(
    texel_copy_byte_size(gpu::TextureFormat::Depth24PlusStencil8, all, size(4, 4)),
    0
  );
}

#[pollster::test]
async fn test_call_count() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let before = gpu.create_call_count_report();

  let usage = gpu::BufferUsages::COPY_SRC | gpu::BufferUsages::COPY_DST;
  let source = create_gpu_buffer_zeroed(256, usage, &gpu.device);
  let target = create_gpu_buffer_zeroed(256, usage, &gpu.device);
  gpu.queue.write_buffer(source.gpu(), 0, &[1; 16]);

  let mut encoder = gpu.create_encoder();
  encoder.copy_buffer_to_buffer(source.gpu(), 0, target.gpu(), 0, 64);
  let mut pass = encoder.begin_compute_pass();
  pass.insert_debug_marker("test");
  assert_eq!(pass.call_counts().debug_marker, 1);
  // the alive pass's counts are not merged yet
  assert_eq!(encoder.call_counts().debug_marker, 0);
  drop(pass);

  let recorded = encoder.call_counts();
  assert_eq!(recorded.copy_buffer_to_buffer, 1);
  assert_eq!(recorded.copy_buffer_to_buffer_bytes, 64);
  assert_eq!(recorded.begin_compute_pass, 1);
  assert_eq!(recorded.debug_marker, 1);

  let report = gpu.create_call_count_report().delta_since(&before);
  assert_eq!(report.device.create_buffer, 2);
  assert_eq!(report.device.create_buffer_bytes, 512);
  assert_eq!(report.device.queue_write_buffer, 1);
  assert_eq!(report.device.queue_write_buffer_bytes, 16);
  assert_eq!(report.device.create_command_encoder, 1);
  assert_eq!(report.submitted_commands, GPUCommandCallCounts::default());

  gpu.submit_encoder(encoder);
  let report = gpu.create_call_count_report().delta_since(&before);
  assert_eq!(report.device.queue_submit, 1);
  assert_eq!(report.device.queue_submit_command_buffers, 1);
  assert_eq!(report.submitted_commands, recorded);
}
