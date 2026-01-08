//! this crate demonstrate a criminal and beautiful hack to implement texture as read-only
//! storage buffer using the powerful `AbstractPtr` mechanism in our shader framework.
//! the reason to do so is to (prototype) support indirect rendering on gles-only platform
//! (should work with the other features like MIDC downgrade and storage buffer auto-merge).

use std::{ops::Range, sync::Arc, vec};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[derive(Clone)]
pub struct TextureAsStorageAllocator(pub GPU);

impl AbstractStorageAllocator for TextureAsStorageAllocator {
  fn allocate_dyn_ty(
    &self,
    byte_size: u64,
    _device: &GPUDevice,
    ty_desc: MaybeUnsizedValueType,
    readonly: bool,
    _label: Option<&str>,
  ) -> BoxedAbstractBuffer {
    assert!(readonly);
    Box::new(TextureAsReadonlyStorageBuffer::new(
      ty_desc,
      byte_size as u32 / 4,
      &self.0.clone(),
    ))
  }

  fn get_layout(&self) -> StructLayoutTarget {
    StructLayoutTarget::Std430
  }

  fn is_readonly(&self) -> bool {
    true
  }
}

#[derive(Clone)]
pub struct TextureAsReadonlyStorageBuffer {
  texture: Arc<RwLock<GPUTypedTextureView<TextureDimension2, u32>>>,
  ty_desc: Arc<MaybeUnsizedValueType>,
  /// this backup is to avoid excessive fragmented texture update call
  ///
  /// we have to do this because we can not use sparse data update
  host_backup: Arc<RwLock<TextureAsReadonlyStorageBufferHostBackup>>,
  gpu: GPU,
}

struct TextureAsReadonlyStorageBufferHostBackup {
  data: Vec<u8>,
  real_data_byte_len: usize,
  dirty_ranges: Vec<Range<usize>>,
  row_byte_len: usize,
}

impl TextureAsReadonlyStorageBufferHostBackup {
  pub fn new(data_byte_size: usize, row_byte_len: usize, array_size: Option<u32>) -> Self {
    assert!(data_byte_size.is_multiple_of(4));
    let _new_size = data_byte_size + 4; // array len;
    let new_size = _new_size.div_ceil(row_byte_len) * row_byte_len;

    let mut r = Self {
      real_data_byte_len: data_byte_size,
      data: vec![0; new_size],
      dirty_ranges: Vec::new(),
      row_byte_len,
    };

    if let Some(array_size) = array_size {
      r.write(bytes_of(&array_size), 0, true);
    }
    r
  }
  pub fn resize(&mut self, data_byte_size: usize, array_size: Option<u32>) {
    let _new_size = data_byte_size + 4; // array len;
    assert!(_new_size.is_multiple_of(4));
    let new_size = _new_size.div_ceil(self.row_byte_len) * self.row_byte_len;
    assert!(new_size >= self.data.len());
    if new_size == self.data.len() {
      return;
    }
    self.dirty_ranges.push(self.data.len()..new_size);
    self.data.resize(new_size, 0);
    self.real_data_byte_len = data_byte_size;

    if let Some(array_size) = array_size {
      self.write(bytes_of(&array_size), 0, true);
    }
  }

  pub fn write(&mut self, data: &[u8], offset: usize, is_array_len: bool) {
    let offset = if is_array_len { offset } else { offset + 4 };
    let range = offset..(offset + data.len());
    self.dirty_ranges.push(range.clone());
    self.data[range].copy_from_slice(data);
  }
}

fn create_tex(
  max_width: u32,
  height: u32,
  gpu: &GPU,
) -> GPUTypedTextureView<TextureDimension2, u32> {
  let texture = GPUTexture::create(
    TextureDescriptor {
      label: None,
      size: Extent3d {
        width: max_width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::R32Uint,
      usage: basic_texture_usages(),
      view_formats: &[],
    },
    &gpu.device,
  );
  texture.create_default_view().try_into().unwrap()
}

impl TextureAsReadonlyStorageBuffer {
  pub fn new_init(ty_desc: MaybeUnsizedValueType, data: &[u8], gpu: &GPU) -> Self {
    let d = Self::new(ty_desc, data.len() as u32 / 4, gpu);
    d.write(data, 0, &gpu.queue);
    d
  }
  pub fn new(ty_desc: MaybeUnsizedValueType, init_u32_size: u32, gpu: &GPU) -> Self {
    // todo, check init_u32_size is valid
    let max_width = gpu.info.supported_limits.max_texture_dimension_2d;
    let height = (init_u32_size + 4).div_ceil(max_width);

    let array_len =
      if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = &ty_desc {
        let size = ty.u32_size_count(StructLayoutTarget::Std430);
        let array_len = init_u32_size / size;
        Some(array_len)
      } else {
        None
      };

    let r = Self {
      texture: Arc::new(RwLock::new(create_tex(max_width, height, gpu))),
      ty_desc: Arc::new(ty_desc),
      host_backup: Arc::new(RwLock::new(TextureAsReadonlyStorageBufferHostBackup::new(
        init_u32_size as usize * 4,
        max_width as usize * 4,
        array_len,
      ))),
      gpu: gpu.clone(),
    };

    if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = &*r.ty_desc {
      let size = ty.u32_size_count(StructLayoutTarget::Std430);
      let array_len = init_u32_size / size;
      r.write(cast_slice(&[array_len]), 0, &gpu.queue);
    }

    r
  }

  #[allow(clippy::needless_range_loop)]
  pub fn check_update_texture(&self) {
    let mut host_backup = self.host_backup.write();
    let texture_row_byte_len = host_backup.row_byte_len;
    let tex_width = texture_row_byte_len as u32 / 4;
    let total_row = host_backup.data.len().div_ceil(texture_row_byte_len);

    let mut dirtied_row = vec![false; total_row];
    for range in host_backup.dirty_ranges.drain(..) {
      let start_row = range.start / texture_row_byte_len;
      let end_row = range.end / texture_row_byte_len;
      for row in start_row..=end_row {
        if end_row == total_row {
          // this happens for last pixel
          continue;
        }
        dirtied_row[row] = true;
      }
    }

    let current_height = self.texture.read().resource.desc.size.height;
    if current_height != total_row as u32 {
      println!("resize texture");
      let new_texture = create_tex(tex_width, total_row as u32, &self.gpu);

      let mut encoder = self.gpu.create_encoder();
      encoder.copy_texture_to_texture(
        TexelCopyTextureInfo {
          texture: self.texture.read().resource.gpu_resource(),
          mip_level: 0,
          origin: Origin3d::ZERO,
          aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
          texture: new_texture.resource.gpu_resource(),
          mip_level: 0,
          origin: Origin3d::ZERO,
          aspect: TextureAspect::All,
        },
        Extent3d {
          width: tex_width,
          height: current_height,
          depth_or_array_layers: 1,
        },
      );
      self.gpu.queue.submit_encoder(encoder);
      *self.texture.write() = new_texture;
    }

    let texture = self.texture.read().resource.gpu_resource().clone();

    for (i, dirty) in dirtied_row.iter().enumerate() {
      if !*dirty {
        continue;
      }

      let data = host_backup
        .data
        .get(i * texture_row_byte_len..(i + 1) * texture_row_byte_len)
        .unwrap();

      self.gpu.queue.write_texture(
        TexelCopyTextureInfo {
          texture: &texture,
          mip_level: 0,
          origin: Origin3d {
            x: 0,
            y: i as u32,
            z: 0,
          },
          aspect: TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: None,  // single row
          rows_per_image: None, // single image
        },
        Extent3d {
          width: tex_width,
          height: 1,
          depth_or_array_layers: 1,
        },
      );
    }
  }
}

impl AbstractBuffer for TextureAsReadonlyStorageBuffer {
  fn write(&self, data: &[u8], offset: u64, _queue: &GPUQueue) {
    self.host_backup.write().write(data, offset as usize, false);
  }

  fn batch_self_relocate(
    &self,
    iter: &mut dyn Iterator<Item = BufferRelocate>,
    _encoder: &mut GPUCommandEncoder,
    _device: &GPUDevice,
  ) {
    println!("TextureAsReadonlyStorageBuffer relocate");
    let relocations: Vec<_> = iter.collect();

    let min = relocations.iter().map(|v| v.self_offset + 4).min().unwrap();
    let max = relocations
      .iter()
      .map(|v| v.self_offset + v.count + 4)
      .max()
      .unwrap();
    let mut host_backup = self.host_backup.write();

    let source_data = host_backup
      .data
      .get(min as usize..max as usize)
      .unwrap()
      .to_vec();

    for r in relocations {
      let old_offset = r.self_offset + 4 - min;
      let data = source_data
        .get(old_offset as usize..(old_offset + r.count) as usize)
        .unwrap();
      host_backup.write(data, r.target_offset as usize, false);
    }
  }

  fn byte_size(&self) -> u64 {
    self.host_backup.read().real_data_byte_len as u64
  }

  fn resize_gpu(
    &mut self,
    _encoder: &mut GPUCommandEncoder,
    _device: &GPUDevice,
    new_byte_size: u64,
  ) {
    let array_len =
      if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) =
        &*self.ty_desc
      {
        let size = ty.u32_size_count(StructLayoutTarget::Std430);
        let array_len = new_byte_size as u32 / 4 / size;
        Some(array_len)
      } else {
        None
      };

    self
      .host_backup
      .write()
      .resize(new_byte_size as usize, array_len);
  }

  fn copy_buffer_to_buffer(
    &self,
    target: &dyn AbstractBuffer,
    self_offset: u64,
    target_offset: u64,
    count: u64,
    _encoder: &mut GPUCommandEncoder,
  ) {
    let target = target.as_any().downcast_ref::<Self>().unwrap();
    let host_backup = self.host_backup.read();
    let self_offset = self_offset as usize + 4;
    let content = host_backup
      .data
      .get(self_offset..(self_offset + count as usize))
      .unwrap();
    target
      .host_backup
      .write()
      .write(content, target_offset as usize, false);
  }

  fn bind_pass(&self, bind_builder: &mut BindingBuilder) {
    self.check_update_texture();
    bind_builder.bind(&*self.texture.read());
  }

  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr {
    let texture = bind_builder.bind_by(&*self.texture.read());

    let mut meta = ShaderU32StructMetaData::new(StructLayoutTarget::Std430);
    meta.register_ty(&self.ty_desc);

    let tex_ptr = TextureAsU32Heap {
      width: texture.texture_dimension_2d(None).x(),
      texture,
    };

    let array_length =
      if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(_)) =
        &*self.ty_desc
      {
        Some(tex_ptr.array_length())
      } else {
        None
      };

    let ptr = U32HeapPtr {
      array: U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(Box::new(tex_ptr))),
      offset: val(0),
    };

    let ptr = U32HeapPtrWithType {
      ptr,
      ty: (*self.ty_desc).clone().into_shader_single_ty(),
      array_length,
      meta: Arc::new(RwLock::new(meta)),
    };

    Box::new(TextureU32HeapPtrWithType { internal: ptr })
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView> {
    None
  }
}

/// wrapped to control access.
#[derive(Clone)]
pub struct TextureU32HeapPtrWithType {
  internal: U32HeapPtrWithType,
}

impl AbstractShaderPtr for TextureU32HeapPtrWithType {
  fn field_index(&self, field_index: usize) -> BoxedShaderPtr {
    self.internal.field_index(field_index)
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    self.internal.field_array_index(index)
  }

  fn array_length(&self) -> Node<u32> {
    self.internal.array_length()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    self.internal.load()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}

/// implement an "array", another way to do so is adding a new trait to abstract
/// the `ShaderReadonlyPtrOf<T>`.
#[derive(Clone)]
struct TextureAsU32Heap {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  width: Node<u32>, // cache the texture dimension call result
}

impl AbstractShaderPtr for TextureAsU32Heap {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let width = self.width;
    let index = index + val(1);
    let x = index % width;
    let y = index / width;
    Box::new(TextureAsU32HeapPosition {
      texture: self.texture,
      position: (x, y).into(),
    })
  }

  fn array_length(&self) -> Node<u32> {
    self.texture.load_texel(val(Vec2::zero()), val(0)).x()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}

#[derive(Clone)]
struct TextureAsU32HeapPosition {
  texture: BindingNode<ShaderTexture<TextureDimension2, u32>>,
  position: Node<Vec2<u32>>,
}

impl AbstractShaderPtr for TextureAsU32HeapPosition {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, _: Node<u32>) -> BoxedShaderPtr {
    unreachable!()
  }

  fn array_length(&self) -> Node<u32> {
    unreachable!()
  }

  fn load(&self) -> ShaderNodeRawHandle {
    self.texture.load_texel(self.position, val(0)).x().handle()
  }

  fn store(&self, _: ShaderNodeRawHandle) {
    unreachable!()
  }

  fn get_self_atomic_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }

  fn get_raw_ptr(&self) -> ShaderNodeRawHandle {
    unreachable!()
  }
}
