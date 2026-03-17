use std::sync::atomic::{AtomicI32, Ordering};

use futures::channel::oneshot::Sender;

use crate::*;

#[derive(Default)]
pub struct GPUxEntityIdMapPicker {
  last_id_buffer_size: Option<Size>,
  wait_to_read_tasks: Vec<(Sender<ReadTextureFromStagingBuffer>, ReadRange)>,
  unresolved_counter: Arc<AtomicI32>,
  // this is to trigger the render when on demand rendering is enabled
  // another way or optimization is to keep a id buffer locally for any time to pick
  waker: Option<Waker>,
}

impl GPUxEntityIdMapPicker {
  pub fn last_id_buffer_size(&self) -> Option<Size> {
    self.last_id_buffer_size
  }
  pub fn read_new_frame_id_buffer(
    &mut self,
    texture: &GPUTypedTextureView<TextureDimension2, u32>,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    waker: &Waker,
  ) {
    let full_size = texture.size();
    self.last_id_buffer_size = Some(full_size);
    for (sender, range) in self.wait_to_read_tasks.drain(..) {
      if let Some(range) = range.clamp(full_size) {
        sender
          .send(encoder.read_texture_2d(&gpu.device, texture, range))
          .ok();
      } // else the sender will drop, and receiver will be resolved
    }
    self.waker = Some(waker.clone());
  }

  pub fn notify_frame_id_buffer_not_available(&mut self) {
    self.wait_to_read_tasks.clear();
    self.last_id_buffer_size = None;
  }

  pub fn pick_point_at(
    &mut self,
    pixel_position: (usize, usize),
  ) -> Option<Box<dyn Future<Output = Option<u32>> + Unpin>> {
    let range = ReadRange {
      size: Size::from_usize_pair_min_one((1, 1)),
      offset_x: pixel_position.0,
      offset_y: pixel_position.1,
    };
    let f = self.pick_ids(range)?;
    let f = f.map(|result| result.map(|ids| ids.first().copied().unwrap_or(0)));

    if let Some(w) = self.waker.take() {
      w.wake();
    }

    Some(Box::new(f))
  }

  /// resolved to None if gpu read failed or read cancelled because of the read range is out of bound.
  ///
  /// - the picking result is not deduplicated
  /// - the result id only contains entity index, without generational info, so it's possible to access
  ///   wrong or deleted entity because of the unsynced entity change happened in same entity position.
  pub fn pick_ids(
    &mut self,
    range: ReadRange,
  ) -> Option<Pin<Box<dyn Future<Output = Option<Vec<u32>>>>>> {
    if self.unresolved_counter.load(Ordering::Relaxed) > 100 {
      return None;
    }

    let counter = self.unresolved_counter.clone();
    counter.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver) = futures::channel::oneshot::channel();
    self.wait_to_read_tasks.push((sender, range));

    Some(Box::pin(
      async {
        let texture_read_future = receiver.await.ok()?;
        let texture_read_buffer = texture_read_future.await.ok()?;
        let buffer = texture_read_buffer.read_into_raw_unpadded_buffer();
        let buffer: &[u32] = bytemuck::cast_slice(&buffer); // todo fix potential alignment issue
        Some(buffer.to_vec())
      }
      .map(move |r| {
        counter.fetch_sub(1, Ordering::Relaxed);
        r
      }),
    ))
  }
}
