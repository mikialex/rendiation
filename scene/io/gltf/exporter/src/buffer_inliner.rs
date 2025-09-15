use crate::*;

pub struct BufferResourceInliner<'a> {
  pub binary_data: &'a mut Option<InlineBinary>,
  pub buffers: &'a mut Resource<u64, gltf_json::Buffer>,
  pub buffer_views: &'a mut Resource<u64, gltf_json::buffer::View>,
}

pub struct InlineBinary {
  pub binary_data: Vec<u8>,
  idx: gltf_json::Index<gltf_json::Buffer>,
}

impl<'a> BufferResourceInliner<'a> {
  pub fn finalize(self) {
    if let Some(binary_data) = &self.binary_data {
      self.buffers.collected[binary_data.idx.value()].byte_length =
        gltf_json::validation::USize64(binary_data.binary_data.len() as u64);
    }
  }

  pub fn collect_inline_packed_view_buffer(
    &mut self,
    buffer: &[u8],
  ) -> gltf_json::Index<gltf_json::buffer::View> {
    let (buffer, byte_length, byte_offset) = self.collect_inline_buffer(buffer);
    self
      .buffer_views
      .append_and_skip_mapping(gltf_json::buffer::View {
        buffer,
        byte_length,
        byte_offset,
        byte_stride: Default::default(),
        name: Default::default(),
        target: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      })
  }

  // return (id, len, offset)
  fn collect_inline_buffer(
    &mut self,
    buffer: &[u8],
  ) -> (
    gltf_json::Index<gltf_json::Buffer>,
    gltf_json::validation::USize64,
    Option<gltf_json::validation::USize64>,
  ) {
    let binary = self.binary_data.get_or_insert_with(|| InlineBinary {
      binary_data: Default::default(),
      idx: self.buffers.append_and_skip_mapping(gltf_json::Buffer {
        byte_length: gltf_json::validation::USize64(0),
        name: Default::default(),
        uri: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
      }),
    });

    // padding to 4, and here we assume the buffer has correct internal padding if required.
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#data-alignment
    while binary.binary_data.len() % 4 != 0 {
      binary.binary_data.push(0);
    }

    let byte_len = buffer.len();
    let byte_offset = binary.binary_data.len();
    binary.binary_data.extend_from_slice(buffer);

    (
      binary.idx,
      gltf_json::validation::USize64(byte_len as u64),
      gltf_json::validation::USize64(byte_offset as u64).into(),
    )
  }
}
