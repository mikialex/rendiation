use crate::*;
use futures::FutureExt;

#[derive(Debug, Copy, Clone)]
pub struct ReadRange {
  pub size: Size,
  pub offset_x: usize,
  pub offset_y: usize,
}

struct ReadTextureTask {
  buffer: wgpu::Buffer,
  inner: futures::channel::oneshot::Receiver<Result<(), BufferAsyncError>>,
}

struct ReadableBuffer {
  // inner: wgpu::BufferSlice,
}

impl ReadableBuffer {
  pub fn read_raw(&self) -> &[u8] {
    // self.inner.
    todo!()
  }
}

use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

impl core::future::Future for ReadTextureTask {
  type Output = Result<ReadableBuffer, wgpu::BufferAsyncError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    match Pin::new(&mut self.inner).poll(cx) {
      Poll::Ready(r) => match r {
        Ok(_) => {
          Poll::Ready(ReadableBuffer {});
        }
        Err(_) => Poll::Ready(Err(wgpu::BufferAsyncError)),
      },
      Poll::Pending => Poll::Pending,
    }
  }
}

struct ReadBufferTask {
  buffer: wgpu::Buffer,
  inner: futures::channel::oneshot::Receiver<Result<(), BufferAsyncError>>,
}

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
    todo!();
  }

  pub fn read_texture_2d(
    &mut self,
    device: &GPUDevice,
    texture: &GPU2DTexture,
    range: ReadRange,
  ) -> ReadTextureTask {
    let (width, height) = range.size.into_usize();
    let buffer_dimensions = BufferDimensions::new(width, height, texture.desc.format);

    // The output buffer lets us retrieve the data as an array
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
      usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    self.encoder.copy_texture_to_buffer(
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

    ReadTextureTask {
      inner: receiver,
      buffer: output_buffer,
    }
  }
}
