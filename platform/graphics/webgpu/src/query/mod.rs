mod time;
pub use time::*;
mod pipeline;
pub use pipeline::*;

use crate::*;

pub(crate) fn read_back_query<T: Pod>(
  query_set: &gpu::QuerySet,
  query_range: Range<u32>,
  device: &GPUDevice,
  encoder: &mut GPUCommandEncoder,
) -> impl Future<Output = Option<T>> + Unpin {
  let size = std::mem::size_of::<T>().max(QUERY_RESOLVE_BUFFER_ALIGNMENT as usize) as u64;
  let usage = BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE;
  let result = create_gpu_buffer_zeroed(size, usage, device).create_default_view();

  encoder.resolve_query_set(query_set, query_range, result.resource.gpu(), 0);

  encoder.read_buffer(device, &result).map(|r| {
    r.ok().map(|r| {
      let view = &r.read_raw()[0..std::mem::size_of::<T>()];
      *bytemuck::from_bytes(view)
    })
  })
}
