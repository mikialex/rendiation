use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct ReadRange {
  pub size: Size,
  pub offset_x: usize,
  pub offset_y: usize,
}

pub struct ReadTextureTask {
  inner: ReadBufferTask,
  info: BufferDimensions,
}

pub struct ReadableTextureBuffer {
  buffer: ReadableBuffer,
  info: BufferDimensions,
}

pub struct ReadableBuffer {
  buffer: wgpu::Buffer,
}

impl ReadableBuffer {
  pub fn read_raw(&self) -> BufferView {
    self.buffer.slice(..).get_mapped_range()
  }
}

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

impl Future for ReadTextureTask {
  type Output = Result<ReadableTextureBuffer, wgpu::BufferAsyncError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    Pin::new(&mut self.inner).poll(cx).map(|r| {
      r.map(|buffer| ReadableTextureBuffer {
        info: self.info,
        buffer,
      })
    })
  }
}

pub struct ReadBufferTask {
  buffer: Option<wgpu::Buffer>,
  inner: futures::channel::oneshot::Receiver<Result<(), BufferAsyncError>>,
}

impl Future for ReadBufferTask {
  type Output = Result<ReadableBuffer, wgpu::BufferAsyncError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match Pin::new(&mut self.inner).poll(cx) {
      Poll::Ready(r) => match r {
        Ok(_) => match self.buffer.take() {
          Some(buffer) => Poll::Ready(Ok(ReadableBuffer { buffer })),
          None => panic!("already resolved"),
        },
        Err(_) => Poll::Ready(Err(wgpu::BufferAsyncError)),
      },
      Poll::Pending => Poll::Pending,
    }
  }
}

#[derive(Copy, Clone)]
struct BufferDimensions {
  width: usize,
  height: usize,
  unpadded_bytes_per_row: usize,
  padded_bytes_per_row: usize,
}

impl BufferDimensions {
  fn new(width: usize, height: usize, format: gpu::TextureFormat) -> Self {
    let bytes_per_pixel = format.describe().block_size as usize;
    let unpadded_bytes_per_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
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
  ) -> ReadBufferTask {
    let size = if let Some(size) = range.size {
      size
    } else {
      buffer.size
    }
    .into();

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_buffer_to_buffer(buffer.gpu.as_ref(), range.offset, &output_buffer, 0, size);

    let buffer_slice = output_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures::channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    ReadBufferTask {
      inner: receiver,
      buffer: Some(output_buffer),
    }
  }

  pub fn read_texture_2d(
    &mut self,
    device: &GPUDevice,
    texture: &GPU2DTexture,
    range: ReadRange,
  ) -> ReadTextureTask {
    let (width, height) = range.size.into_usize();
    let buffer_dimensions = BufferDimensions::new(width, height, texture.desc.format);

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.copy_texture_to_buffer(
      texture.as_image_copy(),
      wgpu::ImageCopyBuffer {
        buffer: &output_buffer,
        layout: wgpu::ImageDataLayout {
          offset: 0,
          bytes_per_row: Some(
            std::num::NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32).unwrap(),
          ),
          rows_per_image: None,
        },
      },
      range.size.into_gpu_size(),
    );

    let buffer_slice = output_buffer.slice(..);
    // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
    let (sender, receiver) = futures::channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    let inner = ReadBufferTask {
      inner: receiver,
      buffer: Some(output_buffer),
    };

    ReadTextureTask {
      inner,
      info: buffer_dimensions,
    }
  }
}
