use rendiation_render_entity::*;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::rc::Rc;
use web_sys::*;

use fnv::FnvHasher;

pub struct BufferManager {
  gl: Rc<WebGlRenderingContext>,
  buffers: HashMap<Rc<BufferData<f32>>, WebGlBuffer, BuildHasherDefault<FnvHasher>>,
  index_buffers: HashMap<Rc<BufferData<u16>>, WebGlBuffer, BuildHasherDefault<FnvHasher>>,
}

impl BufferManager {

  pub fn new(gl: Rc<WebGlRenderingContext>) -> BufferManager{
    BufferManager{
      gl,
      buffers: HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default()),
      index_buffers: HashMap::with_hasher(BuildHasherDefault::<FnvHasher>::default()),
    }
  }

  pub fn get_index_buffer(&mut self, data: Rc<BufferData<u16>>) -> Result<&WebGlBuffer, String> {
    let gl = self.gl.clone();
    Ok(self.index_buffers.entry(data.clone()).or_insert_with(||{
      let buffer = gl.create_buffer().ok_or("failed to create buffer").unwrap();
      gl.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&buffer));
      unsafe {
        let vert_array = js_sys::Uint16Array::view(&data.data);

        gl.buffer_data_with_array_buffer_view(
          WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
          &vert_array,
          WebGlRenderingContext::STATIC_DRAW,
        );
      };
      buffer
    }))
  }

  pub fn get_buffer(&mut self, data: Rc<BufferData<f32>>) -> Result<&WebGlBuffer, String> {
    let gl = self.gl.clone();
    Ok(self.buffers.entry(data.clone()).or_insert_with(||{
      let buffer = gl.create_buffer().ok_or("failed to create buffer").unwrap();
      gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));

      // Note that `Float32Array::view` is somewhat dangerous (hence the
      // `unsafe`!). This is creating a raw view into our module's
      // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
      // (aka do a memory allocation in Rust) it'll cause the buffer to change,
      // causing the `Float32Array` to be invalid.
      //
      // As a result, after `Float32Array::view` we have to be very careful not to
      // do any memory allocations before it's dropped.
      unsafe {
        let vert_array = js_sys::Float32Array::view(&data.data);

        gl.buffer_data_with_array_buffer_view(
          WebGlRenderingContext::ARRAY_BUFFER,
          &vert_array,
          WebGlRenderingContext::STATIC_DRAW,
        );
      };
      buffer
    }))
  }
}
