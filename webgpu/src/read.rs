use __core::ops::RangeBounds;
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
  pub fn read_raw(&self) -> BufferView {
    self.buffer.read_raw()
  }
  pub fn size_info(&self) -> TextReadBufferInfo {
    self.info
  }
}

pub struct ReadableBuffer {
  buffer: gpu::Buffer,
}

impl ReadableBuffer {
  pub fn read_raw(&self) -> BufferView {
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
  inner: futures::channel::oneshot::Receiver<Result<(), BufferAsyncError>>,
}

impl ReadBufferTask {
  pub fn new<S: RangeBounds<BufferAddress>>(buffer: gpu::Buffer, range: S) -> Self {
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

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
}

impl TextReadBufferInfo {
  fn new(width: usize, height: usize, format: gpu::TextureFormat) -> Self {
    let bytes_per_pixel = format.describe().block_size as usize;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = gpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
    let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
    Self {
      width,
      height,
      unpadded_bytes_per_row,
      padded_bytes_per_row,
    }
  }
}

impl GPUCommandEncoder {
  pub fn read_buffer(
    &mut self,
    device: &GPUDevice,
    buffer: &GPUBuffer,
    range: GPUBufferViewRange,
  ) -> ReadBufferFromStagingBuffer {
    let size = if let Some(size) = range.size {
      size
    } else {
      buffer.size
    }
    .into();

    let output_buffer = device.create_buffer(&gpu::BufferDescriptor {
      label: None,
      size,
      usage: gpu::BufferUsages::MAP_READ | gpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_buffer_to_buffer(buffer.gpu.as_ref(), range.offset, &output_buffer, 0, size);

    self
      .on_submit
      .once_future()
      .then(|_| ReadBufferTask::new(output_buffer, ..))
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
        origin: Origin3d {
          x: range.offset_x as u32,
          y: range.offset_y as u32,
          z: 0,
        },
        aspect: TextureAspect::All,
      },
      gpu::ImageCopyBuffer {
        buffer: &output_buffer,
        layout: gpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: Some(
            std::num::NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32).unwrap(),
          ),
          rows_per_image: None,
        },
      },
      range.size.into_gpu_size(),
    );

    self.on_submit.once_future().then(move |_| {
      ReadBufferTask::new(output_buffer, ..).map(move |r| {
        r.map(move |buffer| ReadableTextureBuffer {
          info: buffer_dimensions,
          buffer,
        })
      })
    })
  }
}

pub type ReadTextureFromStagingBuffer =
  impl Future<Output = Result<ReadableTextureBuffer, gpu::BufferAsyncError>>;

pub type ReadBufferFromStagingBuffer =
  impl Future<Output = Result<ReadableBuffer, gpu::BufferAsyncError>>;
