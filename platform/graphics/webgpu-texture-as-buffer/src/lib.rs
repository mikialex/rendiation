//! Texture as readonly storage buffer, to support the storage buffer based features (for example
//! the indirect rendering with MIDC downgrade and the combined buffer) on the platforms that can
//! not use storage buffer in vertex shader, for example gles and webgl2.
//!
//! The data is stored as a u32 heap in a R32Uint 2D texture, and is accessed in shader by the
//! `AbstractShaderPtr` mechanism. Two containers are provided, they share the same texture layout
//! and shader access:
//!
//! - [TextureAsReadonlyStorageBuffer] directly operates on the texture. The behavior mirrors the
//!   gpu buffer implementation, suitable for the data that is written in bulk.
//! - [TextureAsReadonlyStorageBufferWithHost] keeps a full host backup and uploads the merged dirty
//!   ranges before binding, suitable for the large amount of fragmented writes.
//!
//! [TextureAsStorageAllocator] allocates either of them by config.

use std::sync::Arc;

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod direct;
mod dirty;
mod host;
mod layout;
mod shader;
mod texture;

pub use direct::*;
use dirty::*;
pub use host::*;
pub use layout::*;
use shader::*;
use texture::*;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct TextureAsStorageAllocator {
  gpu: GPU,
  host_backup: bool,
  max_width: Option<u32>,
}

impl TextureAsStorageAllocator {
  /// the allocated buffer directly operates on the texture, see [TextureAsReadonlyStorageBuffer]
  pub fn new(gpu: &GPU) -> Self {
    Self {
      gpu: gpu.clone(),
      host_backup: false,
      max_width: None,
    }
  }

  /// the allocated buffer keeps a host backup, see [TextureAsReadonlyStorageBufferWithHost]
  pub fn new_with_host_backup(gpu: &GPU) -> Self {
    Self {
      host_backup: true,
      ..Self::new(gpu)
    }
  }

  /// limit the texture width, the device max texture dimension is used by default
  pub fn with_max_width(mut self, max_width: u32) -> Self {
    self.max_width = Some(max_width);
    self
  }
}

impl AbstractStorageAllocator for TextureAsStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
    label: &str,
  ) -> BoxedAbstractBuffer {
    assert!(readonly, "texture as storage buffer is readonly");
    let limit = device_texel_limit(device, self.max_width);
    if self.host_backup {
      Box::new(TextureAsReadonlyStorageBufferWithHost::new(
        &self.gpu, byte_size, ty_desc, limit, label,
      ))
    } else {
      Box::new(TextureAsReadonlyStorageBuffer::new(
        &self.gpu, byte_size, ty_desc, limit, label,
      ))
    }
  }

  fn get_layout(&self) -> StructLayoutTarget {
    StructLayoutTarget::Std430
  }

  fn is_readonly(&self) -> bool {
    true
  }
}

/// the max texel extent supported by the device, the width can be further limited
pub fn device_texel_limit(device: &GPUDevice, max_width: Option<u32>) -> TexelExtent {
  let max = device.info().supported_limits.max_texture_dimension_2d;
  TexelExtent {
    width: max_width.map_or(max, |w| w.min(max)).max(1),
    height: max,
  }
}

/// the info shared by the ref clones of one buffer
#[derive(Clone)]
struct HeapMeta {
  ty_desc: Arc<MaybeUnsizedValueType>,
  label: Arc<str>,
  limit: TexelExtent,
}

impl HeapMeta {
  fn new(ty_desc: MaybeUnsizedValueType, label: &str, limit: TexelExtent) -> Self {
    Self {
      ty_desc: Arc::new(ty_desc),
      label: label.into(),
      limit,
    }
  }

  /// return None if the size exceeds the limit
  fn required_extent(&self, byte_size: u64) -> Option<TexelExtent> {
    assert!(byte_size.is_multiple_of(4));
    // one more texel for the array length
    TexelExtent::required(byte_size / 4 + 1, self.limit)
  }

  fn required_extent_or_panic(&self, byte_size: u64) -> TexelExtent {
    self.required_extent(byte_size).unwrap_or_else(|| {
      panic!(
        "texture as storage buffer <{}> exceeds the texture size limit, requested {} bytes, the limit is {} bytes",
        self.label,
        byte_size,
        (self.limit.texel_count() - 1) * 4
      )
    })
  }

  fn array_length(&self, byte_size: u64) -> Option<u32> {
    runtime_array_length(&self.ty_desc, byte_size)
  }
}

fn check_range(byte_offset: u64, byte_count: u64, byte_size: u64) {
  assert!(byte_offset.is_multiple_of(4) && byte_count.is_multiple_of(4));
  assert!(
    byte_offset + byte_count <= byte_size,
    "access out of bound, range {}..{}, byte size {}",
    byte_offset,
    byte_offset + byte_count,
    byte_size
  );
}

fn check_relocation(r: &BufferRelocate, src_byte_size: u64, dst_byte_size: u64) {
  check_range(r.self_offset, r.count, src_byte_size);
  check_range(r.target_offset, r.count, dst_byte_size);
}
