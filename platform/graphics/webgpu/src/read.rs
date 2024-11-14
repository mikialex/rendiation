use core::ops::RangeBounds;

use futures::FutureExt;

use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct ReadRange {
  pub size: Size,
  pub offset_x: usize,
  pub offset_y: usize,
}

pub struct ReadableTextureBuffer {
  buffer: ReadableBuffer,
  info: TextReadBufferInfo,
}

impl ReadableTextureBuffer {
  pub fn read_raw(&self) -> gpu::BufferView {
    self.buffer.read_raw()
  }
  pub fn info(&self) -> TextReadBufferInfo {
    self.info
  }
}

pub struct ReadableBuffer {
  buffer: gpu::Buffer,
}

impl ReadableBuffer {
  pub fn read_raw(&self) -> gpu::BufferView {
    self.buffer.slice(..).get_mapped_range()
  }
}

impl Drop for ReadableBuffer {
  fn drop(&mut self) {
    self.buffer.unmap();
  }
}

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

pub struct ReadBufferTask {
  buffer: Option<gpu::Buffer>,
  inner: futures::channel::oneshot::Receiver<Result<(), gpu::BufferAsyncError>>,
}

impl ReadBufferTask {
  pub fn new<S: RangeBounds<gpu::BufferAddress>>(buffer: gpu::Buffer, range: S) -> Self {
    let buffer_slice = buffer.slice(range);
    let (sender, receiver) = futures::channel::oneshot::channel();
    buffer_slice.map_async(gpu::MapMode::Read, move |v| sender.send(v).unwrap());

    Self {
      inner: receiver,
      buffer: Some(buffer),
    }
  }
}

impl Future for ReadBufferTask {
  type Output = Result<ReadableBuffer, gpu::BufferAsyncError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    match Pin::new(&mut self.inner).poll(cx) {
      Poll::Ready(r) => match r {
        Ok(_) => match self.buffer.take() {
          Some(buffer) => Poll::Ready(Ok(ReadableBuffer { buffer })),
          None => panic!("already resolved"),
        },
        Err(_) => unreachable!("actually not canceled"),
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
  fn new(width: usize, height: usize, format: gpu::TextureFormat) -> Self {
    let bytes_per_pixel = format.block_copy_size(None).unwrap() as usize;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = gpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
    let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
    Self {
      width,
      height,
      unpadded_bytes_per_row,
      padded_bytes_per_row,
      format,
    }
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

    #[cfg(test)]
    let device = device.clone();

    self.on_submit.once_future(|_| {}).then(move |_| {
      let r = ReadBufferTask::new(output_buffer, ..);
      #[cfg(test)]
      device.poll(Maintain::Wait);
      r
    })
  }

  pub fn read_buffer_bytes(
    &mut self,
    device: &GPUDevice,
    buffer: &GPUBufferResourceView,
  ) -> impl Future<Output = Result<Vec<u8>, gpu::BufferAsyncError>> {
    self
      .read_buffer(device, buffer)
      .map(|buffer| buffer.map(|buffer| from_bytes_into_boxed_slice(&buffer.read_raw()).into_vec()))
  }

  pub fn read_storage_array<T: Std430>(
    &mut self,
    device: &GPUDevice,
    buffer: &StorageBufferDataView<[T]>,
  ) -> impl Future<Output = Result<Vec<T>, gpu::BufferAsyncError>> {
    self.read_buffer(device, buffer).map(|buffer| {
      buffer.map(|buffer| <[T]>::from_bytes_into_boxed(&buffer.read_raw()).into_vec())
    })
  }

  pub fn read_sized_storage_buffer<T: Std430>(
    &mut self,
    device: &GPUDevice,
    buffer: &StorageBufferDataView<T>,
  ) -> impl Future<Output = Result<T, gpu::BufferAsyncError>> {
    self
      .read_buffer(device, buffer)
      .map(|buffer| buffer.map(|buffer| T::from_bytes(&buffer.read_raw())))
  }

  pub fn read_texture_2d(
    &mut self,
    device: &GPUDevice,
    texture: &GPU2DTexture,
    range: ReadRange,
  ) -> ReadTextureFromStagingBuffer {
    let (width, height) = range.size.into_usize();
    let buffer_dimensions = TextReadBufferInfo::new(width, height, texture.desc.format);

    let output_buffer = device.create_buffer(&gpu::BufferDescriptor {
      label: None,
      size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
      usage: gpu::BufferUsages::MAP_READ | gpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_texture_to_buffer(
      gpu::ImageCopyTexture {
        texture,
        mip_level: 0,
        origin: gpu::Origin3d {
          x: range.offset_x as u32,
          y: range.offset_y as u32,
          z: 0,
        },
        aspect: gpu::TextureAspect::All,
      },
      gpu::ImageCopyBuffer {
        buffer: &output_buffer,
        layout: gpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: Some(buffer_dimensions.padded_bytes_per_row as u32),
          rows_per_image: None,
        },
      },
      range.size.into_gpu_size(),
    );

    #[cfg(test)]
    let device = device.clone();
    self.on_submit.once_future(|_| {}).then(move |_| {
      let r = ReadBufferTask::new(output_buffer, ..).map(move |r| {
        r.map(move |buffer| ReadableTextureBuffer {
          info: buffer_dimensions,
          buffer,
        })
      });

      #[cfg(test)]
      device.poll(Maintain::Wait);
      r
    })
  }
}

pub type ReadTextureFromStagingBuffer =
  impl Future<Output = Result<ReadableTextureBuffer, gpu::BufferAsyncError>> + 'static;

pub type ReadBufferFromStagingBuffer =
  impl Future<Output = Result<ReadableBuffer, gpu::BufferAsyncError>> + 'static;
