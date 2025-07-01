//! this crate demonstrate a criminal and beautiful hack to implement texture as read-only
//! storage buffer using the powerful `AbstractPtr` mechanism in our shader framework.
//! the reason to do so is to (prototype) support indirect rendering on gles-only platform
//! (should work with the other features like MIDC downgrade and storage buffer auto-merge).

use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

pub struct TextureAsReadonlyStorageBuffer<T> {
  phantom: PhantomData<T>,
  texture: GPUTypedTextureView<TextureDimension2, u32>,
  ty_desc: MaybeUnsizedValueType,
}

impl<T> TextureAsReadonlyStorageBuffer<T>
where
  T: Std430MaybeUnsized + ShaderSizedValueNodeType,
{
  pub fn new(ty_desc: MaybeUnsizedValueType, init_u32_size: u32, gpu: &GPU) -> Self {
    // todo, check init_u32_size is valid
    let max_width = gpu.info.supported_limits.max_texture_dimension_2d;
    let required_size = init_u32_size + 1; // add one for storage array length info
    let height = required_size.div_ceil(max_width);

    let texture = GPUTexture::create(
      TextureDescriptor {
        label: None,
        size: Extent3d {
          width: max_width,
          height,
          depth_or_array_layers: 1,
        },
        mip_level_count: 0,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R32Uint,
        usage: basic_texture_usages(),
        view_formats: &[],
      },
      &gpu.device,
    );

    let r = Self {
      phantom: PhantomData,
      texture: texture.create_default_view().try_into().unwrap(),
      ty_desc,
    };

    if let MaybeUnsizedValueType::Unsized(ShaderUnSizedValueType::UnsizedArray(ty)) = &r.ty_desc {
      let size = ty.u32_size_count(StructLayoutTarget::Std430);
      r.write(gpu, &[size], init_u32_size / size);
    }

    r
  }

  /// ```txt
  /// xxxxxoooo  <- head
  /// ooooooooo  <- body (maybe multi row)
  /// ooooooooo  <--|
  /// oooooxxxx. <- tail
  /// ```
  pub fn write(&self, gpu: &GPU, data: &[u32], offset: u32) {
    let width = self.texture.resource.desc.size.width;
    let start_x = offset % width;
    let start_y = offset / width;

    let end = offset + data.len() as u32;
    let end_x = end % width;
    let end_y = end / width;

    let has_head = start_x != 0;
    let has_tail = end_x != 0;
    let has_body = start_y + 1 != end_y;

    let first_row_len = (width - start_x) as usize;
    let last_row_len = end_x as usize;

    if has_head {
      copy_within_row(
        gpu,
        &self.texture.resource,
        data.get(0..first_row_len).unwrap(),
        start_y,
        start_x,
        offset,
      );
    }

    if has_body {
      let bytes_per_row = self
        .texture
        .resource
        .desc
        .format
        .block_copy_size(None)
        .unwrap()
        * width;

      gpu.queue.write_texture(
        TexelCopyTextureInfo {
          texture: self.texture.resource.gpu_resource(),
          mip_level: 0,
          origin: Origin3d {
            x: 0,
            y: start_y + 1,
            z: 0,
          },
          aspect: TextureAspect::All,
        },
        bytemuck::cast_slice(
          data
            .get(first_row_len..(data.len() - last_row_len))
            .unwrap(),
        ),
        TexelCopyBufferLayout {
          offset: (offset + (width - start_x)) as u64,
          bytes_per_row: Some(bytes_per_row),
          rows_per_image: None, // single image
        },
        Extent3d {
          width: data.len() as u32,
          height: 1,
          depth_or_array_layers: 1,
        },
      );
    }

    if has_tail {
      copy_within_row(
        gpu,
        &self.texture.resource,
        data.get((data.len() - last_row_len)..data.len()).unwrap(),
        end_y,
        0,
        offset + data.len() as u32 - end_x,
      );
    }

    fn copy_within_row(
      gpu: &GPU,
      tex: &GPUTexture,
      data: &[u32],
      row: u32,
      row_start: u32,
      offset: u32,
    ) {
      gpu.queue.write_texture(
        TexelCopyTextureInfo {
          texture: tex.gpu_resource(),
          mip_level: 0,
          origin: Origin3d {
            x: row_start,
            y: row,
            z: 0,
          },
          aspect: TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        TexelCopyBufferLayout {
          offset: offset as u64,
          bytes_per_row: None,  // single row
          rows_per_image: None, // single image
        },
        Extent3d {
          width: data.len() as u32,
          height: 1,
          depth_or_array_layers: 1,
        },
      );
    }
  }

  pub fn build_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> ShaderReadonlyPtrOf<T> {
    let texture = bind_builder.bind_by(&self.texture);
    let ptr = U32HeapPtrWithType {
      ptr: U32HeapPtr {
        array: U32HeapHeapSource::Common(<[u32]>::create_view_from_raw_ptr(Box::new(
          TextureAsU32Heap { texture },
        ))),
        offset: val(0),
      },
      ty: self.ty_desc.clone().into_shader_single_ty(),
      bind_index: val(0),
      meta: Arc::new(RwLock::new(ShaderU32StructMetaData::new(
        StructLayoutTarget::Std430,
      ))),
    };

    let ptr = Box::new(TextureU32HeapPtrWithType { internal: ptr });
    T::create_readonly_view_from_raw_ptr(ptr)
  }

  pub fn bind_pass(&mut self, bind_builder: &mut BindingBuilder) {
    bind_builder.bind(&self.texture);
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
}

impl AbstractShaderPtr for TextureAsU32Heap {
  fn field_index(&self, _: usize) -> BoxedShaderPtr {
    unreachable!()
  }

  fn field_array_index(&self, index: Node<u32>) -> BoxedShaderPtr {
    let width = self.texture.texture_dimension_2d(None).x();
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
