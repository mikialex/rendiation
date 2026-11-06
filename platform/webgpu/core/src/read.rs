use core::ops::RangeBounds;

use futures::FutureExt;
use futures::future::{Either, ready};

use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct ReadRange {
  pub size: Size,
  pub offset_x: usize,
  pub offset_y: usize,
}

impl ReadRange {
  pub fn clamp(self, full_size: Size) -> Option<Self> {
    let (full_width, full_height) = full_size.into_usize();
    let (read_width, read_height) = self.size.into_usize();

    if self.offset_x > full_width || self.offset_y > full_height {
      return None;
    }

    let max_x = self.offset_x + read_width;
    let max_y = self.offset_y + read_height;

    let max_x_clamped = max_x.min(full_width);
    let max_y_clamped = max_y.min(full_height);

    let width_clamped = max_x_clamped.checked_sub(self.offset_x)?;
    let height_clamped = max_y_clamped.checked_sub(self.offset_y)?;

    let size = Size::from_usize_pair_min_one((width_clamped, height_clamped));

    Self {
      size,
      offset_x: self.offset_x,
      offset_y: self.offset_y,
    }
    .into()
  }
}

pub struct ReadableTextureBuffer {
  buffer: ReadableBuffer,
  info: TextReadBufferInfo,
}

impl ReadableTextureBuffer {
  pub fn info(&self) -> TextReadBufferInfo {
    self.info
  }
  pub fn read_raw(&self) -> gpu::BufferView {
    self.buffer.read_raw()
  }
  pub fn read_raw_unpadded_bytes_slices(&self, reader: &mut impl FnMut(&[u8])) {
    let padded_buffer = self.read_raw();
    let info = self.info();
    for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
      reader(&chunk[..info.unpadded_bytes_per_row]);
    }
  }

  pub fn read_into_raw_unpadded_buffer(&self) -> Vec<u8> {
    let info = self.info();
    let mut buffer = Vec::with_capacity(info.unpadded_bytes_per_row * info.height);
    self.read_raw_unpadded_bytes_slices(&mut |chunk| buffer.extend_from_slice(chunk));
    buffer
  }
}

pub struct ReadableBuffer {
  buffer: gpu::Buffer,
  counters: Arc<GPUCallCounters>,
}

impl ReadableBuffer {
  pub fn read_raw(&self) -> gpu::BufferView {
    self.counters.device.buffer_get_mapped_range.inc();
    self.buffer.slice(..).get_mapped_range()
  }
}

impl Drop for ReadableBuffer {
  fn drop(&mut self) {
    self.counters.device.buffer_unmap.inc();
    self.buffer.unmap();
  }
}

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

pub struct ReadBufferTask {
  buffer: Option<gpu::Buffer>,
  counters: Arc<GPUCallCounters>,
  inner: futures::channel::oneshot::Receiver<Result<(), gpu::BufferAsyncError>>,
}

impl ReadBufferTask {
  pub fn new<S: RangeBounds<gpu::BufferAddress>>(
    device: &GPUDevice,
    buffer: gpu::Buffer,
    range: S,
  ) -> Self {
    let buffer_slice = buffer.slice(range);
    let counters = device.inner.counters.clone();
    counters.device.buffer_map_async.inc();
    counters
      .device
      .buffer_map_async_bytes
      .add(buffer_slice.size().get());
    let (sender, receiver) = futures::channel::oneshot::channel();
    // the receiver may be dropped if the read task is canceled, panic here will kill the polling
    // thread, so we just ignore the send error.
    buffer_slice.map_async(gpu::MapMode::Read, move |v| {
      sender.send(v).ok();
    });

    Self {
      inner: receiver,
      buffer: Some(buffer),
      counters,
    }
  }
}

impl Future for ReadBufferTask {
  type Output = Result<ReadableBuffer, gpu::BufferAsyncError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    match Pin::new(&mut self.inner).poll(cx) {
      Poll::Ready(r) => match r {
        Ok(Err(err)) => Poll::Ready(Err(err)),
        Ok(Ok(_)) => match self.buffer.take() {
          Some(buffer) => Poll::Ready(Ok(ReadableBuffer {
            buffer,
            counters: self.counters.clone(),
          })),
          None => panic!("already resolved"),
        },
        // the map callback is dropped without being called, for example the device is lost
        Err(_) => Poll::Ready(Err(gpu::BufferAsyncError)),
      },
      Poll::Pending => Poll::Pending,
    }
  }
}

#[derive(Copy, Clone)]
pub struct TextReadBufferInfo {
  pub width: usize,
  pub height: usize,
  pub unpadded_bytes_per_row: usize,
  pub padded_bytes_per_row: usize,
  pub format: gpu::TextureFormat,
}

impl TextReadBufferInfo {
  pub fn size(&self) -> Size {
    Size::from_usize_pair_min_one((self.width, self.height))
  }
}

impl TextReadBufferInfo {
  fn new(
    width: usize,
    height: usize,
    format: gpu::TextureFormat,
    aspect: gpu::TextureAspect,
  ) -> Result<Self, TextureFormatNotCopyable> {
    let bytes_per_pixel = format
      .block_copy_size(Some(aspect))
      .ok_or(TextureFormatNotCopyable { format, aspect })? as usize;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = gpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
    let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
    Ok(Self {
      width,
      height,
      unpadded_bytes_per_row,
      padded_bytes_per_row,
      format,
    })
  }
}

impl GPUCommandEncoder {
  pub fn read_buffer(
    &mut self,
    device: &GPUDevice,
    buffer: &GPUBufferResourceView,
  ) -> ReadBufferFromStagingBuffer {
    let size = buffer.view_byte_size().into();

    let output_buffer = device.create_buffer(&gpu::BufferDescriptor {
      label: None,
      size,
      usage: gpu::BufferUsages::MAP_READ | gpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_buffer_to_buffer(
      &buffer.buffer.gpu,
      buffer.range.offset,
      &output_buffer,
      0,
      size,
    );

    Box::new(self.map_read_after_submit(device, output_buffer))
  }

  /// the map can only be requested after the copy command is submitted. If the encoder is dropped
  /// without submit, the read will resolve as an error.
  fn map_read_after_submit(
    &mut self,
    device: &GPUDevice,
    output_buffer: gpu::Buffer,
  ) -> impl Future<Output = Result<ReadableBuffer, gpu::BufferAsyncError>> + Send + Sync + Unpin + use<>
  {
    let device = device.clone();
    self
      .on_submit
      .wait()
      .then(move |submitted| match submitted {
        Ok(_) => Either::Left(ReadBufferTask::new(&device, output_buffer, ..)),
        Err(_) => Either::Right(ready(Err(gpu::BufferAsyncError))),
      })
  }

  pub fn read_buffer_bytes(
    &mut self,
    device: &GPUDevice,
    buffer: &GPUBufferResourceView,
  ) -> impl Future<Output = Result<Vec<u8>, gpu::BufferAsyncError>> + use<> {
    self
      .read_buffer(device, buffer)
      .map(|buffer| buffer.map(|buffer| from_bytes_into_boxed_slice(&buffer.read_raw()).into_vec()))
  }

  pub fn read_storage_array<T: Std430>(
    &mut self,
    device: &GPUDevice,
    buffer: &StorageBufferDataView<[T]>,
  ) -> impl Future<Output = Result<Vec<T>, gpu::BufferAsyncError>> + use<T> {
    self.read_buffer(device, buffer).map(|buffer| {
      buffer.map(|buffer| <[T]>::from_bytes_into_boxed(&buffer.read_raw()).into_vec())
    })
  }

  pub fn read_atomic_storage_array<T: AtomicityShaderNodeType>(
    &mut self,
    device: &GPUDevice,
    buffer: &StorageBufferDataView<[DeviceAtomic<T>]>,
  ) -> impl Future<Output = Result<Vec<T>, gpu::BufferAsyncError>> + use<T> {
    self.read_buffer(device, buffer).map(|buffer| {
      buffer.map(|buffer| <[T]>::from_bytes_into_boxed(&buffer.read_raw()).into_vec())
    })
  }

  pub fn read_sized_storage_buffer<T: Std430>(
    &mut self,
    device: &GPUDevice,
    buffer: &StorageBufferDataView<T>,
  ) -> impl Future<Output = Result<T, gpu::BufferAsyncError>> + use<T> {
    self
      .read_buffer(device, buffer)
      .map(|buffer| buffer.map(|buffer| T::from_bytes(&buffer.read_raw())))
  }

  /// # Panics
  ///
  /// Panics if the texture format is not copyable, use [GPUCommandEncoder::try_read_texture_2d]
  /// to handle the error.
  pub fn read_texture_2d<F>(
    &mut self,
    device: &GPUDevice,
    texture: &GPUTypedTextureView<TextureDimension2, F>,
    range: ReadRange,
  ) -> ReadTextureFromStagingBuffer {
    expect_copyable(self.try_read_texture_2d(device, texture, range))
  }

  pub fn try_read_texture_2d<F>(
    &mut self,
    device: &GPUDevice,
    texture: &GPUTypedTextureView<TextureDimension2, F>,
    range: ReadRange,
  ) -> Result<ReadTextureFromStagingBuffer, TextureFormatNotCopyable> {
    let (width, height) = range.size.into_usize();
    let format = texture.resource.desc.format;
    // copy requires a single aspect, so we resolve the combined depth stencil format to depth
    let aspect = match texture.desc.aspect {
      gpu::TextureAspect::All if format.is_combined_depth_stencil_format() => {
        gpu::TextureAspect::DepthOnly
      }
      aspect => aspect,
    };
    let buffer_dimensions = TextReadBufferInfo::new(width, height, format, aspect)?;

    let output_buffer = device.create_buffer(&gpu::BufferDescriptor {
      label: None,
      size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
      usage: gpu::BufferUsages::MAP_READ | gpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_texture_to_buffer(
      gpu::TexelCopyTextureInfo {
        texture: texture.resource.gpu_resource().gpu(),
        mip_level: texture.desc.base_mip_level,
        origin: gpu::Origin3d {
          x: range.offset_x as u32,
          y: range.offset_y as u32,
          z: texture.desc.base_array_layer,
        },
        aspect,
      },
      gpu::TexelCopyBufferInfo {
        buffer: &output_buffer,
        layout: gpu::TexelCopyBufferLayout {
          offset: 0,
          bytes_per_row: Some(buffer_dimensions.padded_bytes_per_row as u32),
          rows_per_image: None,
        },
      },
      range.size.into_gpu_size(),
    );

    Ok(Box::new(
      self
        .map_read_after_submit(device, output_buffer)
        .map(move |r| {
          r.map(|buffer| ReadableTextureBuffer {
            info: buffer_dimensions,
            buffer,
          })
        }),
    ))
  }
}

pub type ReadTextureFromStagingBuffer = Box<
  dyn Future<Output = Result<ReadableTextureBuffer, gpu::BufferAsyncError>>
    + Send
    + Sync
    + Unpin
    + 'static,
>;

pub type ReadBufferFromStagingBuffer = Box<
  dyn Future<Output = Result<ReadableBuffer, gpu::BufferAsyncError>>
    + Send
    + Sync
    + Unpin
    + 'static,
>;

#[cfg(test)]
#[pollster::test]
async fn test_read_buffer_with_encoder_dropped() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let usage = gpu::BufferUsages::COPY_SRC;
  let buffer = create_gpu_buffer_zeroed(16, usage, &gpu.device).create_default_view();

  let mut encoder = gpu.create_encoder();
  let read = encoder.read_buffer(&gpu.device, &buffer);
  drop(encoder);

  assert!(read.await.is_err());
}
